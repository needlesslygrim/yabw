use anyhow::anyhow;
use std::ffi::OsStr;
use std::io::{stderr, Write};
use std::path::Path;
use std::process::{Command, Output};

use crate::download::{MediaType, YtDlp, YTDLPJSON};
use crate::Config;

fn parse_json(output: Output) -> anyhow::Result<YTDLPJSON> {
    match serde_json::from_slice::<YTDLPJSON>(&output.stdout) {
        Ok(t) => Ok(t),
        Err(e) => Err(anyhow!("JSON serialisation failed, error: {}", e)),
    }
}

fn process_video(ytdlp: &YtDlp) -> anyhow::Result<()> {
    println!("Processing video...");

    let output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(ytdlp.filename.to_owned())
        .arg("-c")
        .arg("copy")
        .current_dir(
            ytdlp
                .filename
                .parent()
                .ok_or_else(|| anyhow!("Couldn't get download dir from file path"))?,
        )
        .arg(
            OsStr::to_str(
                ytdlp
                    .filename
                    .file_stem()
                    .ok_or_else(|| anyhow!("Couldn't get file name without extension"))?,
            )
            .ok_or_else(|| {
                anyhow!("Couldn't convert filename without extension into normal string")
            })?
            .to_string()
                + ".mp4",
        )
        .output()?;

    if let Some(code) = output.status.code() {
        if code != 0 {
            return Err(anyhow!(
                "ffmpeg failed to execute, the error code was {code}"
            ));
        }
    }
    println!("Processed video.");
    Ok(())
}

fn process_audio(ytdlp: &YtDlp) -> anyhow::Result<()> {
    println!("Processing audio...");

    let output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(ytdlp.filename.to_owned())
        // .arg("-c:a")
        // .arg("copy")
        .current_dir(
            ytdlp
                .filename
                .parent()
                .ok_or_else(|| anyhow!("Couldn't get download dir from file path"))?,
        )
        .arg(
            OsStr::to_str(
                ytdlp
                    .filename
                    .file_stem()
                    .ok_or_else(|| anyhow!("Couldn't get file name without extension"))?,
            )
            .ok_or_else(|| {
                anyhow!("Couldn't convert filename without extension into normal string")
            })?
            .to_string()
                + ".mp3",
        )
        .output()?;
    stderr().write(&output.stderr);
    if let Some(code) = output.status.code() {
        if code != 0 {
            return Err(anyhow!(
                "ffmpeg failed to execute, the error code was {code}"
            ));
        }
    }
    println!("Processed audio.");
    Ok(())
}

fn process_file(ytdlp: &YtDlp, media_type: MediaType) -> anyhow::Result<()> {
    match media_type {
        MediaType::Audio => process_audio(ytdlp),
        MediaType::Video(_) => process_video(ytdlp),
    }
}

fn process_yt_dlp_stdout(output: Output, download_dir: &Path) -> anyhow::Result<YtDlp> {
    let ytdlpjson = parse_json(output)?;
    let ytdlp = YtDlp::new(&ytdlpjson)?;

    Ok(ytdlp)
}

pub fn process(output: Output, config: &Config) -> anyhow::Result<()> {
    let ytdlp = process_yt_dlp_stdout(output, config.download_dir)?;
    let needs_processing = ytdlp.needs_processing(&config)?;

    if needs_processing {
        process_file(&ytdlp, config.media_type.to_owned())?
    }

    Ok(())
}
