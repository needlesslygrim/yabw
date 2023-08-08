fn main() {
    if let Result::Err(err) = youtube_downloader::run() {
        eprintln!("\n----------");
        eprintln!("[-] ERROR: {err}");
        eprintln!("----------");
        std::process::exit(1);
    };
}
