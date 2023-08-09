use console::style;

fn main() {
    if let Result::Err(err) = youtube_downloader::run() {
        eprintln!("{}", style("----------").red());
        eprintln!(
            "{} {err}",
            style("[-] ERROR:").red(),
            err = style(err).red()
        );
        eprintln!("{}", style("----------").red());
        std::process::exit(1);
    };
}
