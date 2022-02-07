#[macro_use]
extern crate log;
use structopt::StructOpt;

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
}

fn main() {
    env_logger::init();

    let args = Args::from_args();
    debug!("{:?}", args);

    let mut channel = vodsync::Channel::new_with_limit(
        args.channel_path.clone(),
        args.url,
        args.hostname,
        args.limit,
    );

    channel.update();

    if let Some(ref channel) = channel.rss_channel {
        if args.write_feed {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(format!("{}.rss", args.channel_path))
                .unwrap();

            channel.pretty_write_to(file, b' ', 2).unwrap();
        } else {
            print!("{}", channel.to_string());
        }
    }
}
