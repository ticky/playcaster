#[macro_use]
extern crate log;
use std::fs::OpenOptions;
use std::path::PathBuf;
use structopt::StructOpt;
use url::Url;

use playcaster::Channel;

#[derive(StructOpt, Debug)]
#[structopt()]
struct Args {
    /// Path to the channel's RSS feed file
    #[structopt(parse(from_os_str))]
    feed_file: PathBuf,
    /// Playlist URL to download videos from
    playlist_url: Url,
    /// Maximum number of videos to download for the given channel
    #[structopt(default_value = "50", long)]
    limit: usize,
    /// Base URL to server which will serve the feed items
    base_url: Url,
    /// Whether to write the updated RSS feed to disk
    #[structopt(long)]
    write_feed: bool,
    /// Additional arguments to be passed to `yt-dlp`
    downloader_arguments: Vec<String>,
}

fn main() {
    env_logger::init();

    let args = Args::from_args();

    println!("Starting up...");

    debug!("{:?}", args);

    let mut channel = Channel::new(args.feed_file.clone(), args.playlist_url);

    println!("Updating channel... (this can take a pretty long time)");

    channel.update_with_args(args.base_url, args.limit, args.downloader_arguments);

    println!(" Done!");

    if let Some(ref channel) = channel.rss_channel {
        if args.write_feed {
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(args.feed_file)
                .expect("Unable to open file for writing");

            channel
                .pretty_write_to(file, b' ', 2)
                .expect("Couldn't write XML to file");
        } else {
            print!("{}", channel.to_string());
        }
    }
}
