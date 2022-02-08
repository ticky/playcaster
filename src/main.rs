#[macro_use]
extern crate log;
use std::fs::OpenOptions;
use structopt::StructOpt;

use playcaster::Channel;

#[derive(StructOpt, Debug)]
#[structopt()]
struct Args {
    /// Path to the channel's folder and RSS feed
    // TODO: #[structopt(parse(from_os_str))]
    channel_path: String, // TODO: PathBuf
    /// URL to download videos from
    url: String,
    /// Maximum number of videos to download for the given channel
    #[structopt(default_value = "50", long)]
    limit: usize,
    /// Hostname of server which will serve the feed items
    hostname: String,
    /// Whether to write the updated RSS feed to disk
    #[structopt(long)]
    write_feed: bool,
    /// Additional arguments to be passed to `yt-dlp`
    downloader_arguments: Vec<String>,
}

fn main() {
    env_logger::init();

    let args = Args::from_args();
    debug!("{:?}", args);

    let mut channel = Channel::new_with_limit(
        args.channel_path.clone(),
        args.url,
        args.hostname,
        args.limit,
    );

    channel.update_with_args(args.downloader_arguments);

    if let Some(ref channel) = channel.rss_channel {
        if args.write_feed {
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(format!("{}.xml", args.channel_path))
                .expect("Unable to open file for writing");

            channel
                .pretty_write_to(file, b' ', 2)
                .expect("Couldn't write XML to file");
        } else {
            print!("{}", channel.to_string());
        }
    }
}
