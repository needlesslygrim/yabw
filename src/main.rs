use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::process::{exit, Command, ExitStatus};

#[derive(Serialize, Deserialize, Debug)]
struct YTDLPJSON {
    requested_downloads: Vec<RequestedDownload>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct RequestedDownload {
    #[serde(rename = "_filename")]
    pub filename: String,
}

fn main() {
    let mut url = String::new();
    io::stdin()
        .read_line(&mut url)
        .expect("Couldn't read input");

    let download_dir = UserDirs::new().unwrap();
    let download_dir = download_dir.download_dir().unwrap();

    let output = Command::new("yt-dlp")
        .arg(url)
        .arg("-J")
        .arg("--no-simulate")
        .current_dir(download_dir)
        .output()
        .expect("yt-dlp failed to start, please check to see you have it installed.");

    if let Some(code) = output.status.code() {
        if code != 0 {
            panic!("Something went wrong with yt-dlp, here's the error code: {code}")
        }
    }
    io::stdout().write(&output.stdout).unwrap();
    let ytdlp: YTDLPJSON = serde_json::from_slice(&output.stdout).unwrap();
    println!("{:#?}", ytdlp)
}
