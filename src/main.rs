fn rss_channel_from(target_url: String, limit: u64) -> rss::Channel {
    let channel = youtube_dl::YoutubeDl::new(target_url.clone())
        .youtube_dl_path("yt-dlp")
        // .extra_arg("--flat-playlist")
        .extra_arg("--playlist-end")
        .extra_arg(limit.to_string())
        .run()
        .unwrap();

    log::debug!("{:#?}", channel);

    if let youtube_dl::YoutubeDlOutput::Playlist(playlist) = channel {
        let title = match playlist.title {
            Some(title) => title,
            None => target_url.to_string(),
        };

        let link = match playlist.webpage_url {
            Some(url) => url,
            None => target_url,
        };

        let description = format!("Vodsync podcast feed for {}", title);

        let rss_itunes_category = rss::extension::itunes::ITunesCategoryBuilder::default()
            .text("TV & Film")
            .build();

        let rss_itunes_extension = rss::extension::itunes::ITunesChannelExtensionBuilder::default()
            .author(title.clone())
            .subtitle(title.clone())
            .summary(description.clone())
            .explicit("No".to_string())
            // TODO: .image
            .category(rss_itunes_category)
            .block("Yes".to_string())
            .build();

        let rss_items: Vec<rss::Item> = match playlist.entries {
            Some(entries) => {
                entries
                    .into_iter()
                    .map(|video| {
                        use hhmmss::Hhmmss;

                        let duration = match video.duration {
                            Some(value) => {
                                let secs = match value {
                                    serde_json::Value::Number(secs) => secs.as_f64().unwrap_or(0.0),
                                    _ => 0.0,
                                };
                                std::time::Duration::new(secs as u64, 0)
                            }
                            None => std::time::Duration::default(),
                        };

                        let upload_date = video.upload_date.map(|date| {
                            chrono::Date::<chrono::Utc>::from_utc(chrono::NaiveDate::parse_from_str(&date, "%Y%m%d").unwrap(), chrono::Utc)
                                .and_hms(0,0,0)
                                .to_rfc2822()
                        });

                        let item_itunes_extension =
                            rss::extension::itunes::ITunesItemExtensionBuilder::default()
                                .author(title.clone())
                                .subtitle(video.title.clone())
                                .summary(video.description)
                                .image(video.thumbnail)
                                .duration(duration.hhmmss())
                                .explicit("No".to_string())
                                .build();

                        rss::ItemBuilder::default()
                            .guid(rss::GuidBuilder::default().value(video.id).build())
                            .title(video.title)
                            .link(video.webpage_url)
                            .pub_date(upload_date)
                            // TODO: .enclosure
                            .itunes_ext(item_itunes_extension)
                            .build()
                    })
                    .collect()
            }
            None => vec![],
        };

        rss::ChannelBuilder::default()
            .title(title)
            .link(link)
            .description(description)
            .generator("Vodsync (https://github.com/ticky/vodsync)".to_string())
            .itunes_ext(rss_itunes_extension)
            .items(rss_items)
            .build()
    } else {
        panic!("This URL points to a single video, not a channel!")
    }
}

fn main() {
    env_logger::init();

    // let target_url = "https://www.twitch.tv/zandravandra/videos";
    let target_url = "https://www.youtube.com/c/mightycarmods";

    print!("{:#?}", rss_channel_from(target_url.to_string(), 10));
}

#[test]
fn test_youtube_rss_valid() {
    use rss::validation::Validate;
    let channel = rss_channel_from("https://www.youtube.com/c/mightycarmods".to_string(), 2);
    channel.validate().unwrap();
}

#[test]
fn test_twitch_rss_valid() {
    use rss::validation::Validate;
    let channel = rss_channel_from("https://www.twitch.tv/zandravandra/videos".to_string(), 2);
    channel.validate().unwrap();
}
