/// Represents a given RSS channel, which points at a video feed.
pub struct Channel {
    name: String,
    target_url: String,
    limit: u16,
    pub rss_channel: Option<rss::Channel>,
}

// TODO:
// - Read channel from disk if already extant
// - Update method which looks at existing episodes to adjust
// - Write channel to disk
// - Download specific episodes
impl Channel {
    pub fn new_with_limit(name: String, target_url: String, limit: u16) -> Self {
        let rss_channel = match std::fs::File::open(format!("{}.rss", name)) {
            Ok(file) => {
                let reader = std::io::BufReader::new(file);
                match rss::Channel::read_from(reader) {
                    Ok(channel) => Some(channel),
                    Err(_) => None,
                }
            },
            Err(_) => None
        };

        Self {
            name,
            target_url,
            limit,
            rss_channel,
        }
    }

    pub fn new(name: String, target_url: String) -> Self {
        Self::new_with_limit(name, target_url, 50)
    }

    pub fn update(&mut self) {
        let ytdl_result = youtube_dl::YoutubeDl::new(self.target_url.clone())
            .youtube_dl_path("yt-dlp")
            // .extra_arg("--flat-playlist") // NOTE: This makes very long playlists work, but misses out on lots of metadata
            .extra_arg("--playlist-end")
            .extra_arg(self.limit.to_string())
            .extra_arg("--format")
            .extra_arg("bestvideo[ext=mp4][vcodec^=avc1]+bestaudio[ext=m4a]/best[ext=mp4][vcodec^=avc1]/best[ext=mp4]/best")
            // .extra_arg("--output")
            // .extra_arg()
            .run()
            .unwrap();

        log::debug!("{:#?}", ytdl_result);

        if let youtube_dl::YoutubeDlOutput::Playlist(playlist) = ytdl_result {
            let title = playlist.title.as_ref().unwrap_or_else(|| { &self.target_url }).clone();

            let rss_items: Vec<rss::Item> = match playlist.entries {
                Some(ref entries) => entries
                    .into_iter()
                    .map(|video| {
                        use hhmmss::Hhmmss;

                        let duration = match &video.duration {
                            Some(value) => {
                                let secs = match value {
                                    serde_json::Value::Number(secs) => secs.as_f64().unwrap_or(0.0),
                                    _ => 0.0,
                                };
                                std::time::Duration::new(secs as u64, 0)
                            }
                            None => std::time::Duration::default(),
                        };

                        let upload_date = video.upload_date.as_ref().map(|date| {
                            chrono::Date::<chrono::Utc>::from_utc(
                                chrono::NaiveDate::parse_from_str(&date, "%Y%m%d").unwrap(),
                                chrono::Utc,
                            )
                            .and_hms(0, 0, 0)
                            .to_rfc2822()
                        });

                        let item_itunes_extension =
                            rss::extension::itunes::ITunesItemExtensionBuilder::default()
                                .author(title.clone())
                                .subtitle(video.title.clone())
                                .summary(video.description.clone())
                                .image(video.thumbnail.clone())
                                .duration(duration.hhmmss())
                                .explicit("No".to_string())
                                .build();

                        let item_enclosure = rss::EnclosureBuilder::default()
                            .url(format!("http://192.168.1.193:8000/{}.mp4", video.id)) // TODO: This has to be absolute!
                            .length((video.filesize_approx.unwrap_or(0.0) as u64).to_string())
                            .mime_type("video/mp4")
                            .build();

                        rss::ItemBuilder::default()
                            .guid(rss::GuidBuilder::default().value(video.id.clone()).build())
                            .title(video.title.clone())
                            .link(video.webpage_url.clone())
                            .pub_date(upload_date)
                            .enclosure(item_enclosure)
                            .itunes_ext(item_itunes_extension)
                            .build()
                    })
                    .collect(),
                None => vec![],
            };

            let mut rss_channel = self.rss_channel.clone().unwrap_or_else(|| {
                let link = match playlist.webpage_url {
                    Some(url) => url,
                    None => self.target_url.to_string(),
                };

                let description = format!("Vodsync podcast feed for {}", title);

                let rss_itunes_category = rss::extension::itunes::ITunesCategoryBuilder::default()
                    .text("TV & Film")
                    .build();

                let rss_itunes_extension =
                    rss::extension::itunes::ITunesChannelExtensionBuilder::default()
                        .author(title.clone())
                        .subtitle(title.clone())
                        .summary(description.clone())
                        .explicit("No".to_string())
                        // TODO: .image
                        .category(rss_itunes_category)
                        .block("Yes".to_string())
                        .build();


                rss::ChannelBuilder::default()
                    .title(title)
                    .link(link)
                    .description(description)
                    .generator("Vodsync (https://github.com/ticky/vodsync)".to_string())
                    .itunes_ext(rss_itunes_extension)
                    .build()
            });

            rss_channel.set_items(rss_items);

            self.rss_channel = Some(rss_channel);
        } else {
            panic!("This URL points to a single video, not a channel!")
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_twitch_rss_valid() {
        use rss::validation::Validate;
        let mut channel = super::Channel::new_with_limit(
            "zandravandra".to_string(),
            "https://www.twitch.tv/zandravandra/videos".to_string(),
            2,
        );
        channel.update();
        channel.rss_channel.unwrap().validate().unwrap();
    }
}
