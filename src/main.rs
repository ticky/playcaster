#[macro_use]
extern crate log;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt()]
struct Args {
    /// Path to the channel's folder and RSS feed
    channel_path: String,
    /// URL to download videos from
    url: String,
    /// Maximum number of videos to download for the given channel
    #[structopt(default_value = "50", long)]
    limit: usize,
    /// Hostname of server which will serve the feed items
    hostname: String
}

fn main() {
    env_logger::init();

    let args = Args::from_args();
    debug!("{:?}", args);

    let mut channel =
        vodsync::Channel::new_with_limit(args.channel_path, args.url, args.hostname, args.limit);

    channel.update();

    print!("{}", channel.rss_channel.unwrap().to_string());
}
