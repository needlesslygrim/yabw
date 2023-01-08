use std::process;

fn main() {
    if let Err(e) = youtube_downloader::run() {
        eprintln!("Application error: {e}");
        process::exit(1);
    };
}
