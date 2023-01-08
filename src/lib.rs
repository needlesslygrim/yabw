use anyhow::{anyhow, Context};
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

#[derive(Serialize, Deserialize, Debug)]
pub struct YTDLPJSON {
    requested_downloads: Vec<RequestedDownload>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequestedDownload {
    #[serde(rename = "_filename")]
    pub filename: String,
}
fn get_url() -> anyhow::Result<String> {
    let mut url = String::new();
    io::stdin().read_line(&mut url)?;
    Ok(url)
}
fn download_video(download_dir: &Path) -> anyhow::Result<(YTDLPJSON, bool)> {
    let url = get_url();

    let output = Command::new("yt-dlp")
        .arg(url?)
        .arg("-J")
        .arg("--no-simulate")
        .current_dir(download_dir)
        .output()?;

    if let Some(code) = output.status.code() {
        if code != 0 {
            return Err(anyhow!(
                "yt-dlp failed to execute, the error code was {code}"
            ));
        }
    }
    io::stdout().write(&output.stdout)?;
    let yt_dlp: YTDLPJSON =
        serde_json::from_slice(&output.stdout).context(anyhow!("JSON serialisation failed"))?;
    println!("{:#?}", yt_dlp);
    let filename = (&yt_dlp)
        .requested_downloads
        .get(0)
        .ok_or(anyhow!("This shouldn't be possible"))?
        .filename
        .to_owned();
    let needs_processing = filename
        .get(filename.len() - 3..)
        .ok_or(anyhow!("This shouldn't be possible"))?
        != "mp4";
    Ok((yt_dlp, needs_processing))
}

pub fn process_video(ytdlp: YTDLPJSON, download_dir: &Path) -> anyhow::Result<()> {
    println!("Processing video...");
    let filename = &ytdlp
        .requested_downloads
        .get(0)
        .ok_or(anyhow!("This shouldn't be possible"))?
        .filename;

    let output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(filename)
        .arg("-c")
        .arg("copy")
        .current_dir(download_dir)
        .arg((&filename)[1..filename.len() - 4].to_owned() + "mp4")
        .output()?;

    if let Some(code) = output.status.code() {
        if code != 0 {
            return Err(anyhow!(
                "ffmpeg failed to execute, the error code was {code}"
            ));
        }
    }

    Ok(())
}

pub fn run() -> anyhow::Result<()> {
    let user_dirs = UserDirs::new().ok_or(anyhow!("Couldn't get user directories."))?;
    let download_dir = user_dirs
        .download_dir()
        .ok_or(anyhow!("Couldn't find downloads directory"))?;

    let (ytdlp, needs_processing) = download_video(download_dir)?;
    if needs_processing {
        return process_video(ytdlp, download_dir);
    }

    Ok(())
}
