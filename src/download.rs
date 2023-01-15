use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{stderr, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::Config;

#[derive(Serialize, Deserialize, Debug)]
pub struct YTDLPJSON {
    pub requested_downloads: Vec<RequestedDownload>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequestedDownload {
    #[serde(rename = "filepath")]
    pub filepath: String,
}
#[derive(Debug, Clone)]
pub struct YtDlp {
    pub filename: PathBuf,
}

impl YtDlp {
    pub fn new(yt_dlp_json: &YTDLPJSON) -> anyhow::Result<Self> {
        Ok(YtDlp {
            filename: Path::new(
                yt_dlp_json
                    .requested_downloads
                    .get(0)
                    .ok_or_else(|| anyhow!("This shouldn't be possible"))?
                    .filepath
                    .as_str(),
            )
            .into(),
        })
    }

    pub fn needs_processing(&self, config: &Config) -> anyhow::Result<bool> {
        Ok(self
            .filename
            .extension()
            .ok_or_else(|| anyhow!("File downloaded does not have a valid extension"))?
            != match config.media_type {
                MediaType::Audio => "mp3",
                MediaType::Video(_) => "mp4",
            })
    }
}

#[derive(Debug, Copy)]
pub enum MediaType {
    Audio,
    Video(Resolution),
}

impl Clone for MediaType {
    fn clone(&self) -> Self {
        match self {
            MediaType::Audio => MediaType::Audio,
            MediaType::Video(resolution) => MediaType::Video(*resolution),
        }
    }
}
#[derive(Debug, Copy)]
pub enum Resolution {
    P0144,
    P0480,
    P0720,
    P1080,
    P1440,
    P2160,
}

impl Clone for Resolution {
    fn clone(&self) -> Self {
        match self {
            Resolution::P0144 => Resolution::P0144,
            Resolution::P0480 => Resolution::P0480,
            Resolution::P0720 => Resolution::P0720,
            Resolution::P1080 => Resolution::P1080,
            Resolution::P1440 => Resolution::P1440,
            Resolution::P2160 => Resolution::P2160,
        }
    }
}

fn download_video(config: &Config) -> anyhow::Result<Output> {
    let MediaType::Video(resolution) = config.media_type else { return Err(anyhow!("tried to download video but the requested type was audio")) };

    let resolution_arg = match resolution {
        Resolution::P2160 => "res:2160",
        Resolution::P1440 => "res:1440",
        Resolution::P1080 => "res:1080",
        Resolution::P0720 => "res:720",
        Resolution::P0480 => "res:480",
        Resolution::P0144 => "res:144",
    };

    let output = Command::new("yt-dlp")
        .arg(&config.url)
        .arg("-J")
        .arg("-q")
        .arg("--no-simulate")
        .arg("-S")
        .arg(resolution_arg)
        .current_dir(config.download_dir)
        .output()?;

    Ok(output)
}

fn download_audio(config: &Config) -> anyhow::Result<Output> {
    let output = Command::new("yt-dlp")
        .arg(&config.url)
        .arg("-J")
        .arg("-q")
        .arg("-x")
        .arg("--no-simulate")
        .current_dir(config.download_dir)
        .output()?;

    Ok(output)
}
pub fn download(config: &Config) -> anyhow::Result<Output> {
    let output = match config.media_type {
        MediaType::Audio => download_audio(config)?,
        MediaType::Video(_) => download_video(config)?,
    };
    File::create("yt-dlp.json")?.write(&output.stdout)?;
    if let Some(code) = output.status.code() {
        if code != 0 {
            return Err(anyhow!(
                "yt-dlp failed to execute, the error message was: \n {}",
                String::from_utf8(output.stderr)?
            ));
        }
    }

    Ok(output)
}
