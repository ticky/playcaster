fn main() {
    env_logger::init();

    // let target_url = "https://www.twitch.tv/zandravandra/videos";
    let target_url = "https://www.youtube.com/c/mightycarmods";
    // let target_url = "QWkUFkXcx9I";

    // TODO: Web server!

    let mut channel =
        vodsync::Channel::new_with_limit("test".to_string(), target_url.to_string(), 1);

    channel.update();

    print!("{}", channel.rss_channel.unwrap().to_string());
}
