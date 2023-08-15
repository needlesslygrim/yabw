use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use directories::UserDirs;
use serde::Deserialize;

use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;

#[cfg(feature = "gui")]
mod gui;

#[derive(Deserialize, Debug, Clone, Default)]
pub struct YtDlpJson {
    pub requested_downloads: Vec<RequestedDownload>,
}

impl YtDlpJson {
    fn get(config: &Config) -> Result<Vec<RequestedDownload>, String> {
        let f = File::create("yt-dlp.log")
            .map_err(|err| format!("Couldn't open the yt-dlp log file: {err}"))?;
        let mut writer = BufWriter::new(f);

        let mut downloads = Vec::with_capacity(config.downloads.len());

        // TODO: Multithreading
        for download in &config.downloads {
            let mut command = make_base_command(download);

            let output = command
                .arg("-J")
                .arg("--no-clean-info-json")
                .arg(&download.url)
                .output()
                .map_err(|err| format!("yt-dlp failed to run: {err}"))?;

            if output.stdout != NULL_YT_DLP_STDOUT {
                writer.write_all(&output.stdout).map_err(|err| {
                    format!("Failed to write `stdout` to the yt-dlp log file: {err}")
                })?;
            }
            writer.write_all(&output.stderr).map_err(|err| {
                format!("Failed to write `stderr` to the yt-dlp log file: {err}`")
            })?;
            let mut requested_downloads = serde_json::from_slice::<YtDlpJson>(&output.stdout)
                .map_err(|err| {
                    format!(
                    "Failed to parse JSON output of yt-dlp, check the log file `yt-dlp.log`: {err}"
                )
                })?
                .requested_downloads;
            downloads.append(&mut requested_downloads);
        }
        Ok(downloads)
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct RequestedDownload {
    filename: PathBuf,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Resolution {
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
pub struct ParseResolutionError;

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

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Default)]
pub enum MediaType {
    #[default]
    Audio,
    Video(Resolution),
}

#[derive(Debug)]
pub struct ParseMediaTypeError;

impl Display for ParseMediaTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to parse media type from usize")
    }
}

impl Error for ParseMediaTypeError {}

#[derive(Debug, Clone)]
pub struct Download<'a> {
    pub url: String,
    pub media_type: MediaType,
    pub filepath: PathBuf,
    pub download_dir: &'a Path,
}

impl<'a> Download<'a> {
    fn download(&self) -> Result<(), String> {
        let mut command = make_base_command(self);

        command
            .arg("--force-overwrites")
            .arg("--quiet")
            .arg("--progress")
            .arg("--newline")
            .arg(&self.url)
            .current_dir(env::temp_dir());

        command
            .spawn()
            .map_err(|err| format!("yt-dlp failed to run: {err}"))?
            .wait()
            .map_err(|err| format!("Failed to wait on yt-dlp: {err}"))?;

        Ok(())
    }

    fn process(mut self) -> Result<(), String> {
        let mut command = process::Command::new("ffmpeg");
        command
            .arg("-i")
            .arg(&self.filepath)
            .args(match self.media_type {
                MediaType::Audio => ["-c:a", "libmp3lame", "-vn"].as_slice(),
                MediaType::Video(_) => {
                    ["-c:v", "libx265", "-preset", "fast", "-c:a", "aac"].as_slice()
                }
            });

        self.filepath.set_extension(match self.media_type {
            MediaType::Audio => "mp3",
            MediaType::Video(_) => "mp4",
        });

        let output = command
            .arg("-y") // Overwrites files that already exist
            .arg(self.filepath.file_name().ok_or(
                "The filename of the video could not be found from the filepath constructed",
            )?)
            .current_dir(self.download_dir)
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

    fn needs_processing(&self) -> Result<bool, &'static str> {
        let extension = self
            .filepath
            .file_name()
            .ok_or("Failed to check if a download needed processing")?;

        Ok(match self.media_type {
            MediaType::Audio => extension != "mp3",
            MediaType::Video(_) => extension != "mp4",
        })
    }
}

#[derive(Debug, Clone)]
pub struct Config<'a> {
    downloads: Vec<Download<'a>>,
}

impl<'a> Config<'a> {
    fn get_interactive(download_dir: &'a Path) -> Result<Self, Box<dyn Error>> {
        let theme = ColorfulTheme::default();

        let url = Input::<String>::with_theme(&theme)
            .with_prompt("Enter the URL of the video to download")
            .interact_text()
            .map_err(|err| format!("Failed to read choice of URL: {err}"))?;

        let media_type: MediaType = match Select::with_theme(&theme)
            .item("Audio")
            .item("Video")
            .with_prompt("Pick a media type")
            .interact()
            .map_err(|err| format!("Failed to read choice of media type: {err}"))?
        {
            0 => Ok(MediaType::Audio),
            1 => Ok(MediaType::Video({
                let mut selector = Select::with_theme(&theme);
                selector
                    .item("144p")
                    .item("240p")
                    .item("480p")
                    .item("720p")
                    .item("1080p")
                    .item("1440p")
                    .item("2160p")
                    .with_prompt("Pick a resolution");

                selector.interact()
                    .map_err(|err| format!("Failed to read choice of resolution: {err}"))?
                    .try_into()
                    .map_err(|_| "Somehow the index of the chosen media type is invalid, just try the tool again")?
            })),
            _ => Err(
                "Somehow the index of the chosen media type is invalid, just try the tool again",
            ),
        }?;

        let download = Download {
            url,
            media_type,
            filepath: Default::default(),
            download_dir,
        };

        Ok(Config {
            downloads: vec![download],
        })
    }
}

const NULL_YT_DLP_STDOUT: [u8; 5] = *b"null\n"; // If yt-dlp fails to run successfully, `stdout` will have had `null\n' written to it.

#[cfg(feature = "gui")]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    gui::App::build()
}

// FIXME: Implement a custom error type to avoid dynamic dispatch.
#[cfg(not(feature = "gui"))]
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
    let requested_downloads = YtDlpJson::get(&config)?; // Get the configuration of `yt-dlp` for this video, only the filename at the moment.
    config
        .downloads
        .iter_mut()
        .zip(
            requested_downloads
                .into_iter()
                .map(|requsted_download| requsted_download.filename),
        )
        .for_each(|(download, filename)| {
            download.filepath = filename;
        });

    println!(
        "{message}",
        message = style("[+]  INFO: Loaded video information.").cyan()
    );

    println!(
        "{message}",
        message = style("[+]  INFO: Downloading file...").cyan()
    );
    for (_, mut download) in config.downloads.into_iter().enumerate() {
        download.download()?;
        let mut filepath = env::temp_dir(); // Get the temp directory...
        filepath.push(download.filepath); // ...and push the filename to it, so that we have a full path to the file.
        download.filepath = filepath; // Then set the config filepath to it.

        if download.needs_processing()? {
            println!(
                "{message}",
                message = style("[+]  INFO: Processing file...").cyan()
            );
            download.process()?;
            println!(
                "{message}",
                message = style("[+]  INFO: File processed.").cyan()
            );
        }
    }
    println!("{message}", message = style("[+]  INFO: The tool has finished running, your file is located in your downloads directory.").green());
    Ok(())
}

fn make_base_command(download: &Download) -> process::Command {
    let mut command = process::Command::new("yt-dlp");

    match download.media_type {
        MediaType::Audio => {
            command.arg("-f").arg("bestaudio");
        }
        MediaType::Video(resolution) => {
            command
                .arg("-S")
                .arg(format!("res:{}", resolution.as_str()));
        }
    };
    command.arg("--no-playlist"); // Currently the tool with crash when trying to parse the JSON output of yt-dlp without this flag.

    command
}
