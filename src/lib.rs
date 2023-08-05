use std::io::{self, Write};
use std::ops::Deref;
use std::path::Path;
use std::process::{Command, Output};
use std::{env, fs, fs::File, path::PathBuf};

use directories::UserDirs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct YtDlpJson {
    pub requested_downloads: Vec<RequestedDownload>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RequestedDownload {
    filename: PathBuf,
}

#[derive(Copy, Clone, Debug)]
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

#[derive(Debug, Clone)]
struct Config<'a> {
    url: String,
    media_type: MediaType,
    resolution: Option<Resolution>,
    filepath: PathBuf,
    download_dir: &'a Path,
}

const NULL_JSON_RESPONSE: [u8; 5] = *b"null\n"; // If yt-dlp fails to run successfully, `stdout` will have had `null\n' written to it.

#[derive(Debug)]
struct ResolutionParseError;

pub fn run() {
    let user_dirs =
        UserDirs::new() // Get the directories of the current user so that we can get the right download directory.
            // TODO: Add an option to specify a download directory.
            .expect("Couldn't get user directories, please use Redox/Linux/Windows/macOS.");
    let download_dir = user_dirs
        .download_dir()
        .expect("Somehow couldn't get download directory.");

    let mut config = get_config(download_dir); // Get the configuration for the tool, resolution, video, etc.
    println!("Loading video information");
    let yt_dlp_config = get_yt_dlp_config(&config); // Find out where `yt-dlp` will store the downloaded file.
    println!("Loaded video information");

    download(&config); // Actually download the video.
    println!("Downloading file.");

    if let Some(filename) = yt_dlp_config
        .requested_downloads
        .first()
        .map(|download| download.filename.clone())
    // TODO: Figure out if this `clone()` is unnecessary.
    {
        // Get the full filepath of the downloaded file.
        let mut filepath = env::temp_dir();
        filepath.push(filename);
        config.filepath = filepath;

        let extension = config
            .filepath
            .extension()
            .expect("Somehow the file downloaded has no file extension?");
        let needs_processing = match config.media_type {
            MediaType::Audio => extension != "mp3",
            MediaType::Video => extension != "mp4",
        };
        if needs_processing {
            println!("Processing file.");
            let output = process(&mut config); // Process the video if it is not already an mp3 or mp4.
            if !output.status.success() {
                let mut f = File::create("ffmpeg.log").expect("Couldn't open log file"); // TODO: Log in the proper location.
                f.write_all(&output.stderr)
                    .expect("Couldn't write `stderr` to log file.");
                panic!("ffmpeg did not run successfully, check the error log.");
            }
            let mut stdout = output.stdout.clone();
            let mut stderr = output.stderr;
            stdout.append(&mut stderr);
            fs::write("ffmpeg.log", stdout).expect("TODO: panic message");
        }
    }
    println!("File processed.");
    println!("The tool has finished running, your file is located in your downloads directory.");
}

fn get_config(download_dir: &Path) -> Config {
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
        filepath: Default::default(),
        download_dir,
    }
}

fn make_base_command(config: &Config) -> Command {
    let mut command = Command::new("yt-dlp");

    match config.media_type {
        MediaType::Audio => {
            command.arg("-f").arg("bestaudio");
        }
        MediaType::Video => {
            command
            .arg("-S")
            .arg(format!("res:{}", config
            .resolution
            .expect("Somehow `config.media_type` is `MediaType::Video`, but `config.resolution` is `None`???")
            .to_string()));
        }
    };

    command
}

fn get_yt_dlp_config(config: &Config) -> YtDlpJson {
    let mut command = make_base_command(config);
    command
        .arg("-J")
        // .arg("--no-simulate") // TODO: Remove
        .arg("--no-clean-info-json")
        .arg(&config.url);

    let output = command
        .output()
        .expect("`yt-dlp` failed to run successfully, check that it is installed");

    parse_yt_dlp_json(&output)
}

fn parse_yt_dlp_json(output: &Output) -> YtDlpJson {
    if !output.status.success() {
        let mut f = File::create("yt-dlp.log").expect("Couldn't open log file"); // TODO: Log in the proper location.
        f.write_all(&output.stderr)
            .expect("Couldn't write `stderr` to log file.");
        if output.stdout != NULL_JSON_RESPONSE {
            f.write_all(&output.stdout)
                .expect("Couldn't write `stdout` to log file.");
        }
        panic!("yt-dlp did not run successfully, check the error log.");
    }
    let mut stdout = output.stdout.clone();
    let mut stderr = output.stderr.clone();
    stdout.append(&mut stderr);
    fs::write("yt-dlp.log", stdout).expect("TODO: panic message");
    let parsed = serde_json::from_slice::<YtDlpJson>(&output.stdout)
        .expect("Failed to parse JSON output of `yt-dlp`");

    parsed
}

fn download(config: &Config) {
    let mut command = make_base_command(config);

    command
        .arg("--force-overwrites")
        .arg("--quiet")
        .arg("--progress")
        .arg("--newline")
        .arg(&config.url)
        .current_dir(env::temp_dir());

    command
        .spawn()
        .expect("`yt-dlp` failed to run, please check that it is installed.")
        .wait()
        .expect("Failed to wait on `yt-dlp`.");
}

fn process(config: &mut Config) -> Output {
    let mut command = Command::new("ffmpeg");
    command
        .arg("-i")
        .arg(&config.filepath)
        .args(match config.media_type {
            MediaType::Audio => ["-c:a", "libmp3lame", "-vn"].as_slice(),
            MediaType::Video => ["-c:a", "copy", "-c:v", "copy"].as_slice(),
        });

    config.filepath.set_extension(match config.media_type {
        MediaType::Audio => "mp3",
        MediaType::Video => "mp4",
    });

    command
        .arg(config.filepath.file_name().expect("Somehow no file name?"))
        .current_dir(config.download_dir);
    command
        .output()
        .expect("Failed to run ffmpeg, please check that it is installed.")
}
