#[macro_use]
extern crate log;
use anyhow::Result;
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

    /// Base URL to server which will serve the feed items
    base_url: Url,

    /// Playlist URL to download videos from.
    /// Required if creating a new feed, or if the feed's link element doesn't already point to a playlist URL.
    #[structopt(long)]
    playlist_url: Option<Url>,

    /// Maximum number of videos to download for the given channel
    #[structopt(default_value = "30", long)]
    limit: usize,

    /// Do not write the updated RSS feed to disk; just print it to the terminal
    #[structopt(long)]
    no_write_feed: bool,

    /// Additional arguments to be passed to `yt-dlp`
    downloader_arguments: Vec<String>,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::from_args();

    println!("Starting up...");

    debug!("{:?}", args);

    let mut channel = match args.playlist_url {
        Some(url) => Channel::new_with_url(args.feed_file.clone(), url),
        None => Channel::new(args.feed_file.clone()),
    }?;

    println!("Updating channel... (this can take a pretty long time)");

    channel.update_with_args(args.base_url, args.limit, args.downloader_arguments)?;

    match channel.rss_channel {
        Some(ref channel) => {
            if args.no_write_feed {
                print!("{:#}", channel.to_string());
            } else {
                let file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(args.feed_file)?;

                channel.pretty_write_to(file, b' ', 2)?;
            }
        }
        None => warn!("No RSS channel generated"),
    }

    println!(" Done!");

    Ok(())
}
