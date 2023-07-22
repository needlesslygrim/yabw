use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::{fs::File, path::PathBuf, process};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct YtDlpJson {
    pub requested_downloads: Vec<RequestedDownload>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RequestedDownload {
    filename: PathBuf,
    #[serde(rename(serialize = "__finadir", deserialize = "__finaldir"))]
    finaldir: PathBuf,
    filepath: PathBuf,
}

#[derive(Copy, Clone)]
enum Resolution {
    P144,
    P240,
    P720,
    P1080,
    P480,
    P1440,
    P2160,
}

impl TryFrom<&str> for Resolution {
    type Error = ResolutionParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "144" | "144P" | "144p" => Ok(Self::P144),
            "240" | "240P" | "240p" => Ok(Self::P240),
            "480" | "480P" | "480p" => Ok(Self::P480),
            "720" | "720P" | "720p" => Ok(Self::P720),
            "1080" | "1080P" | "1080p" => Ok(Self::P1080),
            "1440" | "1440P" | "1440p" => Ok(Self::P1440),
            "2160" | "2160P" | "2160p" => Ok(Self::P2160),
            _ => Err(ResolutionParseError),
        }
    }
}

impl ToString for Resolution {
    fn to_string(&self) -> String {
        match self {
            Self::P144 => String::from("144"),
            Self::P240 => String::from("240"),
            Self::P480 => String::from("480"),
            Self::P720 => String::from("720"),
            Self::P1080 => String::from("1080"),
            Self::P1440 => String::from("1440"),
            Self::P2160 => String::from("2160"),
        }
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
enum MediaType {
    Audio,
    Video,
}

struct Config {
    url: String,
    media_type: MediaType,
    resolution: Option<Resolution>,
}

const NULL_JSON_RESPONSE: [u8; 5] = *b"null\n"; // If yt-dlp fails to run successfully, `stdout` will have had `null\n' written to it.

#[derive(Debug)]
struct ResolutionParseError;

pub fn run() -> YtDlpJson {
    let config = get_config();
    let output = download(&config);
    if !output.status.success() {
        let mut f = File::create("output.json").expect("Couldn't open log file");
        f.write_all(&output.stderr)
            .expect("Couldn't write `stderr` to log file.");
        if output.stdout != NULL_JSON_RESPONSE {
            f.write_all(&output.stdout)
                .expect("Couldn't write `stdout` to log file.");
        }
        panic!("yt-dlp did not run successfully, check the error log.");
    }

    serde_json::from_slice::<YtDlpJson>(&output.stdout).expect("This shouldn't have happened, but somehow the JSON failed to be parsed into a `ytDlpJson` struct.")
}

fn get_config() -> Config {
    let mut url = String::with_capacity(43); // Maybe the normal length of a youtube url?(?)
    println!("Enter the url to download, e.g. 'https://www.youtube.com/watch?v=2hXNd6x9sZs'.");
    io::stdin().read_line(&mut url).expect("Couldn't read URL.");

    println!("Would you like to download the video and audio or just the audio? (v/a)");
    let mut media_type = String::new();
    io::stdin()
        .read_line(&mut media_type)
        .expect("Couldn't read choice of media type.");

    let media_type = match media_type.trim() {
        "v" => MediaType::Video,
        "a" => MediaType::Audio,
        _ => panic!("Invalid media type"),
    };

    let mut resolution = None;
    if media_type == MediaType::Video {
        println!("Please enter a resolution");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Could not read resolution");
        resolution = Some(Resolution::try_from(input.trim()).expect("Invalid resolution"));
    }

    Config {
        url: String::from(url.trim()),
        media_type,
        resolution,
    }
}

fn download(config: &Config) -> process::Output {
    let mut command = process::Command::new("yt-dlp");

    if let Some(resolution) = config.resolution {
        command
            .arg("-S")
            .arg(format!("res:{}", resolution.to_string()));
    } else {
        command.arg("-x");
    }

    command
        .arg("-J")
        .arg("--no-simulate")
        .arg("--force-overwrites")
        .arg(&config.url);

    command
        .output()
        .expect("`yt-dlp` failed to run, check that it is installed.")
}
