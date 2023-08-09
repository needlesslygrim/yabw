use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use directories::UserDirs;
use serde::{Deserialize, Serialize};

use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct YtDlpJson {
    pub requested_downloads: Vec<RequestedDownload>,
}

impl YtDlpJson {
    fn get(config: &Config) -> Result<YtDlpJson, String> {
        let mut command = make_base_command(config);

        let output = command
            .arg("-J")
            .arg("--no-clean-info-json")
            .arg(&config.url)
            .output()
            .map_err(|err| format!("yt-dlp failed to run: {err}"))?;

        let mut f = File::create("yt-dlp.log")
            .map_err(|err| format!("Couldn't open the yt-dlp log file: {err}"))?;
        if output.stdout != NULL_YT_DLP_STDOUT {
            f.write_all(&output.stdout)
                .map_err(|err| format!("Failed to write `stdout` to the yt-dlp log file: {err}"))?;
        }
        f.write_all(&output.stderr)
            .map_err(|err| format!("Failed to write `stderr` to the yt-dlp log file: {err}`"))?;

        serde_json::from_slice(&output.stdout).map_err(|err| {
            format!("Failed to parse JSON output of yt-dlp, check the log file `yt-dlp.log`: {err}")
        })
    }
}

// TODO: Find out if we can store the resolution, media type, and url of the download in here.
// Doing so would require using `#[serde(skip_serializing)]` so that `serde` doesn't complain.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RequestedDownload {
    filename: PathBuf,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Resolution {
    P144,
    P240,
    P720,
    P1080,
    P480,
    P1440,
    P2160,
}

impl Resolution {
    fn as_str(&self) -> &'static str {
        match self {
            Self::P144 => "144",
            Self::P240 => "240",
            Self::P480 => "480",
            Self::P720 => "720",
            Self::P1080 => "1080",
            Self::P1440 => "1440",
            Self::P2160 => "2160",
        }
    }
}

#[derive(Debug)]
struct ParseResolutionError;

impl Display for ParseResolutionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to parse resolution from usize")
    }
}

impl Error for ParseResolutionError {}

// I know this doesn't really make much sense, resolution is numeric, however,
// I only use this functionality in one place, where we have an index, so too bad :^).
impl TryFrom<usize> for Resolution {
    type Error = ParseResolutionError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::P144),
            1 => Ok(Self::P240),
            2 => Ok(Self::P480),
            3 => Ok(Self::P720),
            4 => Ok(Self::P1080),
            5 => Ok(Self::P1440),
            6 => Ok(Self::P2160),
            _ => Err(ParseResolutionError),
        }
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
enum MediaType {
    Audio,
    Video,
}

impl TryFrom<usize> for MediaType {
    type Error = ParseMediaTypeError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MediaType::Audio),
            1 => Ok(MediaType::Video),
            _ => Err(ParseMediaTypeError),
        }
    }
}

#[derive(Debug)]
struct ParseMediaTypeError;

impl Display for ParseMediaTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to parse media type from usize")
    }
}

impl Error for ParseMediaTypeError {}

#[derive(Debug, Clone)]
struct Config<'a> {
    url: String,
    media_type: MediaType,
    resolution: Option<Resolution>,
    filepath: PathBuf,
    download_dir: &'a Path,
}

impl<'a> Config<'a> {
    fn get_interactive(download_dir: &'a Path) -> Result<Self, Box<dyn Error>> {
        let theme = ColorfulTheme::default();

        let url = Input::<String>::with_theme(&theme)
            .with_prompt("Enter the URL of the video to download")
            .interact_text()
            .map_err(|err| format!("Failed to read choice of URL: {err}"))?;

        let media_type: MediaType = Select::with_theme(&theme)
            .item("Audio")
            .item("Video")
            .with_prompt("Pick a media type")
            .interact()
            .map_err(|err| format!("Failed to read choice of media type: {err}"))?
            .try_into()
            .map_err(|_| {
                "Somehow the index of the chosen media type is invalid, just try the tool again"
            })?;

        let resolution = match media_type {
            MediaType::Audio => None,
            MediaType::Video => {
                Some(Select::with_theme(&theme)
                    .item("144p")
                    .item("240p")
                    .item("480p")
                    .item("720p")
                    .item("1080p")
                    .item("1440p")
                    .item("2160p")
                    .with_prompt("Pick a resolution")
                    .interact()
                    .map_err(|err| format!("Failed to read choice of resolution: {err}"))?
                    .try_into()
                    .map_err(|_| "Somehow the index of the chosen media type is invalid, just try the tool again")?)
            }
        };

        Ok(Config {
            url: String::from(url.trim()),
            media_type,
            resolution,
            filepath: Default::default(),
            download_dir,
        })
    }
}

const NULL_YT_DLP_STDOUT: [u8; 5] = *b"null\n"; // If yt-dlp fails to run successfully, `stdout` will have had `null\n' written to it.

// FIXME: Implement a custom error type to avoid dynamic dispatch.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let user_dirs = UserDirs::new()
        .ok_or("Couldn't get user directories, please use Redox/Linux/Windows/macOS")?; // Get the directories of the current user so that we can get the right download directory.
                                                                                        // TODO: Add an command line flag to specify a download directory.
    let download_dir = user_dirs
        .download_dir()
        .ok_or("Could not find your Downloads directory, if you're using Linux please make sure that your XDG environment variables are set")?;

    let mut config = Config::get_interactive(download_dir)?; // Get the configuration for the tool interactively.

    println!(
        "{message}",
        message = style("[+]  INFO: Loading video information...").cyan()
    );
    let yt_dlp_config = YtDlpJson::get(&config)?; // Get the JSON configuration of `yt-dlp` for this video, just filename at the moment.
    println!(
        "{message}",
        message = style("[+]  INFO: Loaded video information.").cyan()
    );

    println!(
        "{message}",
        message = style("[+]  INFO: Downloading file...").cyan()
    );
    download(&config)?; // Actually download the video.

    let filename = yt_dlp_config
        .requested_downloads
        .first()
        .map(|download| &download.filename).ok_or("The output from `yt-dlp` parsed successfully, but the `requested_fields` field of `yt-dlp-config` is an empty Vec")?; // Get the filename of the download.

    let mut filepath = env::temp_dir(); // Get the temp directory...
    filepath.push(filename); // ...and push the filename to it, so that we have a full path to the file.
    config.filepath = filepath; // Then set the config filepath to it.

    let extension = config.filepath.extension().ok_or(
        "The file was downloaded successfully by `yt-dlp` however it has no file extension",
    )?; // Get the extension of the file so that we can determine whether we need to process it or not.

    let needs_processing = match config.media_type {
        MediaType::Audio => extension != "mp3",
        MediaType::Video => extension != "mp4",
    };

    if needs_processing {
        println!(
            "{message}",
            message = style("[+]  INFO: Processing file...").cyan()
        );
        process(&mut config)?;
        println!(
            "{message}",
            message = style("[+]  INFO: File processed.").cyan()
        );
    }
    println!("{message}", message = style("[+]  INFO: The tool has finished running, your file is located in your downloads directory.").green());
    Ok(())
}

fn make_base_command(config: &Config) -> process::Command {
    let mut command = process::Command::new("yt-dlp");

    match config.media_type {
        MediaType::Audio => {
            command.arg("-f").arg("bestaudio");
        }
        MediaType::Video => {
            command
            .arg("-S")
            .arg(format!("res:{}", config
            .resolution
            .expect("`config.media_type` is `MediaType::Video`, but `config.resolution` is `None`, which shouldn't be possible")
            .as_str()));
        }
    };
    command.arg("--no-playlist"); // Currently the tool with crash when trying to parse the JSON output of yt-dlp without this flag.

    command
}

fn download(config: &Config) -> Result<(), String> {
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
        .map_err(|err| format!("yt-dlp failed to run: {err}"))?
        .wait()
        .map_err(|err| format!("Failed to wait on yt-dlp: {err}"))?;

    Ok(())
}

fn process(config: &mut Config) -> Result<(), String> {
    let mut command = process::Command::new("ffmpeg");
    command
        .arg("-i")
        .arg(&config.filepath)
        .args(match config.media_type {
            MediaType::Audio => ["-c:a", "libmp3lame", "-vn"].as_slice(),
            MediaType::Video => ["-c:v", "libx265", "-preset", "fast", "-c:a", "aac"].as_slice(),
        });

    config.filepath.set_extension(match config.media_type {
        MediaType::Audio => "mp3",
        MediaType::Video => "mp4",
    });

    let output =
        command
            .arg("-y") // Overwrites files that already exist
            .arg(config.filepath.file_name().ok_or(
                "The filename of the video could not be found from the filepath constructed",
            )?)
            .current_dir(config.download_dir)
            .output()
            .map_err(|err| format!("Failed to run ffmpeg: {err}"))?;

    let mut f = File::create("ffmpeg.log")
        .map_err(|err| format!("Couldn't open the ffmpeg log file: {err}"))?;
    f.write_all(&output.stdout)
        .map_err(|err| format!("Failed to write `stdout` to the ffmpeg log file: {err}"))?;
    f.write_all(&output.stderr)
        .map_err(|err| format!("Failed to write `stderr` to the ffmpeg log file: {err}"))?;

    Ok(())
}
