mod download;
mod input;
mod processing;

use anyhow::anyhow;
use directories::UserDirs;
use download::{download, MediaType, Resolution};
use std::io;
use std::path::Path;

use processing::process;

pub struct Config<'a> {
    media_type: MediaType,
    download_dir: &'a Path,
    url: String,
}

impl<'a> Config<'a> {
    fn new(media_type: MediaType, download_dir: &'a Path, url: Option<String>) -> Self {
        Config {
            media_type,
            download_dir,
            url: url.unwrap_or(String::new()),
        }
    }
}

fn get_url() -> anyhow::Result<String> {
    println!("Please enter the URL of the content you would like to download");
    let mut url = String::new();
    io::stdin().read_line(&mut url)?;
    Ok(url)
}

pub fn run() -> anyhow::Result<()> {
    let user_dirs = UserDirs::new().ok_or_else(|| anyhow!("Couldn't get user directories."))?;
    let download_dir = user_dirs
        .download_dir()
        .ok_or_else(|| anyhow!("Couldn't find downloads directory"))?;

    let url = get_url()?;

    let config = Config::new(MediaType::Video(Resolution::P0144), download_dir, Some(url));

    let output = download(&config)?;

    process(output, &config)?;
    // if needs_processing {
    //     return process_video(ytdlp, download_dir);
    // }
    Ok(())
}
