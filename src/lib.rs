use serde_derive::Deserialize;

#[derive(Deserialize)]
struct Config {
    server: ServerConfig,
    feeds: std::collections::HashMap<String, FeedConfig>,
    downloader: DownloaderConfig,
}

#[derive(Deserialize)]
struct DownloaderConfig {
    self_update: bool,
    timeout: Option<u32>
}

fn default_page_size() -> u16 { 50 }
fn default_update_period() -> String { "6h".to_string() }

#[derive(Deserialize)]
struct FeedConfig {
    url: String,
    #[serde(default = "default_page_size")]
    page_size: u16,
    #[serde(default = "default_update_period")]
    update_period: String,
    youtube_dl_args: Vec<String>,
}

#[derive(Deserialize)]
struct ServerConfig {
    hostname: Option<String>,
    bind_address: Option<String>,
    port: u32,
    data_dir: String,
}

#[cfg(test)]
mod test {
    #[test]
    fn test_config_parser() {
        // This configuration is for PodSync, but we aim to be compatible
        let config: super::Config = toml::from_str(r#"
            [server]
            hostname = "http://connie.local:8080"
            port = 8080
            # Don't change if you run podsync via docker
            data_dir = "/app/data"

            [tokens]
            # YouTube API Key. See https://developers.google.com/youtube/registering_an_application
            youtube = "aaaaaaaaa"

            [feeds]
              [feeds.chipcheezumlps]
              url = "https://www.youtube.com/user/ChipCheezumLPs"
              page_size = 10
              update_period = "6h"
              quality = "high"
              format = "video"
              clean = { keep_last = 30 }
              filters = { not_title = "Cut Commentary" }
              opml = true
              youtube_dl_args = [ "--write-sub", "--write-auto-sub", "--embed-subs", "--sub-lang", "en" ]

            [downloader]
            self_update = true # Optional, auto update youtube-dl every 24 hours

            [database]
            badger = { truncate = true, file_io = true }
        "#).unwrap();

        assert_eq!(config.server.hostname, Some("http://connie.local:8080".to_string()));
        assert_eq!(config.server.bind_address, None);
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.data_dir, "/app/data");

        assert_eq!(
            config.feeds["chipcheezumlps"].url,
            "https://www.youtube.com/user/ChipCheezumLPs"
        );
        assert_eq!(config.feeds["chipcheezumlps"].page_size, 10);
        assert_eq!(config.feeds["chipcheezumlps"].update_period, "6h");
        assert_eq!(
            config.feeds["chipcheezumlps"].youtube_dl_args,
            [
                "--write-sub",
                "--write-auto-sub",
                "--embed-subs",
                "--sub-lang",
                "en"
            ]
        );

        assert_eq!(config.downloader.self_update, true);
        assert_eq!(config.downloader.timeout, None);
    }
}
