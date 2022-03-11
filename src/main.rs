#[macro_use]
extern crate log;
use anyhow::Result;
use std::fs::OpenOptions;
use std::path::PathBuf;
use clap::Parser;
use url::Url;

use playcaster::Channel;

#[derive(Parser, Debug)]
#[clap(version)]
/// Turn any playlist into a Podcast feed
struct Args {
    /// Path to the channel's RSS feed file
    #[clap(parse(from_os_str))]
    feed_file: PathBuf,

    /// Base URL to server which will serve the feed items
    base_url: Url,

    /// Playlist URL to download videos from.
    /// Required if creating a new feed, or if the feed's link element doesn't already point to a playlist URL.
    #[clap(long)]
    playlist_url: Option<Url>,

    /// Maximum number of videos to download for the given channel
    #[clap(default_value = "30", long)]
    limit: usize,

    /// Maximum number of videos to keep for the given channel.
    /// Any older videos will be deleted when the feed updates.
    /// Should be greater than or equal to `limit`.
    #[clap(long)]
    keep: Option<usize>,

    /// Do not write the updated RSS feed to disk; just print it to the terminal
    #[clap(long)]
    no_write_feed: bool,

    /// Write terse RSS XML to disk, rather than the default pretty-printed version
    #[clap(long)]
    no_pretty: bool,

    /// Additional arguments to be passed to `yt-dlp`
    downloader_arguments: Vec<String>,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    println!("Starting up...");

    trace!("{:?}", args);

    let mut channel = match args.playlist_url {
        Some(url) => Channel::new_with_url(args.feed_file.clone(), url),
        None => Channel::new(args.feed_file.clone()),
    }?;

    println!("Updating channel... (this can take a pretty long time)");

    channel.update_with_args(args.base_url, args.limit, args.keep, args.downloader_arguments)?;

    match channel.rss_channel {
        Some(ref rss_channel) => {
            if args.no_write_feed {
                print!("{:#}", rss_channel.to_string());
            } else {
                let file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(args.feed_file)?;

                if args.no_pretty {
                    rss_channel.write_to(file)?;
                } else {
                    rss_channel.pretty_write_to(file, b' ', 2)?;
                }
            }
        }
        None => warn!("No RSS channel generated"),
    }

    println!("Done!");

    Ok(())
}
