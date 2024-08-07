#[macro_use]
extern crate log;

use chrono::{NaiveDateTime, TimeZone, Utc};

use itertools::Itertools;

use rss::extension::itunes::{
    ITunesCategoryBuilder, ITunesChannelExtensionBuilder, ITunesItemExtensionBuilder,
};
use rss::{
    Channel as RSSChannel, ChannelBuilder as RSSChannelBuilder,
    EnclosureBuilder as RSSEnclosureBuilder, GuidBuilder as RSSGuidBuilder, Item as RSSItem,
    ItemBuilder as RSSItemBuilder,
};

use url::Url;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error as ThisError;

use youtube_dl::{YoutubeDl, YoutubeDlOutput};

/// Wrapper error types
#[derive(ThisError, Debug)]
pub enum Error {
    /// Error case where a `std::io::Error` was encountered while reading or writing to disk
    #[error("I/O error")]
    IoError(#[from] std::io::Error),

    /// Error case where an `rss::Error` was encountered while parsing the RSS feed
    #[error("RSS feed error")]
    FeedError(#[from] rss::Error),

    /// Error case where a `url::ParseError` was encountered
    #[error("URL parsing error")]
    UrlError(#[from] url::ParseError),

    /// Error case where a `youtube_dl::Error` was encountered
    #[error("error in downloader")]
    YtDlError(#[from] youtube_dl::Error),

    /// Error case where all target files were zero-duration after downloading
    #[error("all entries in \"{0}\" had a zero duration. This likely means the target playlist was a playlist of other playlists")]
    AllDownloadsEmptyError(Url),

    /// Error case the supplied `feed_file` path was invalid
    #[error("invalid feed file path: \"{0}\"")]
    ParentPathError(PathBuf),

    /// Error case the supplied `feed_file` path was invalid due to not having a name
    #[error("invalid feed file path: \"{0}\" (file must have a name)")]
    FileStemError(PathBuf),

    /// Error case the supplied `feed_file` path was invalid due to not having an extension
    #[error(
        "invalid feed file path: \"{0}\" (file must have an extension - \"xml\" is a good one!)"
    )]
    FileExtensionError(PathBuf),
}

pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_DOWNLOAD_LIMIT: usize = 30;

/// Represents a given RSS channel, which points at a video feed.
pub struct Channel {
    /// Path to the input RSS feed
    pub feed_file: PathBuf,

    /// URL to the playlist with videos to be downloaded
    pub playlist_url: Url,

    /// The RSS feed
    pub rss_channel: Option<RSSChannel>,
}

impl Channel {
    pub fn new_with_reader_and_url<T: BufRead>(
        feed_file: PathBuf,
        playlist_url: Url,
        reader: T,
    ) -> Result<Self, Error> {
        if feed_file.extension().is_none() {
            Err(Error::FileExtensionError(feed_file))
        } else {
            // Don't pull the URL out of the RSS channel

            Ok(Self {
                feed_file,
                playlist_url,
                rss_channel: RSSChannel::read_from(reader).ok(),
            })
        }
    }

    pub fn new_with_reader<T: BufRead>(feed_file: PathBuf, reader: T) -> Result<Self, Error> {
        if feed_file.extension().is_none() {
            Err(Error::FileExtensionError(feed_file))
        } else {
            let rss_channel = RSSChannel::read_from(reader)?;

            let playlist_url = Url::parse(rss_channel.link())?;

            Ok(Self {
                feed_file,
                playlist_url,
                rss_channel: Some(rss_channel),
            })
        }
    }

    pub fn new_with_url(feed_file: PathBuf, playlist_url: Url) -> Result<Self, Error> {
        if feed_file.extension().is_none() {
            Err(Error::FileExtensionError(feed_file))
        } else if let Ok(file) = File::open(feed_file.clone()) {
            let reader = BufReader::new(file);
            Self::new_with_reader_and_url(feed_file, playlist_url, reader)
        } else {
            Ok(Self {
                feed_file,
                playlist_url,
                rss_channel: None,
            })
        }
    }

    pub fn new(feed_file: PathBuf) -> Result<Self, Error> {
        if feed_file.extension().is_none() {
            Err(Error::FileExtensionError(feed_file))
        } else {
            let file = File::open(feed_file.clone())?;
            let reader = BufReader::new(file);
            Self::new_with_reader(feed_file, reader)
        }
    }

    fn update_with_playlist(
        &mut self,
        base_url: Url,
        keep: Option<usize>,
        playlist: youtube_dl::Playlist,
    ) -> Result<(), Error> {
        let title = playlist
            .title
            .as_ref()
            .unwrap_or(&self.playlist_url.to_string())
            .clone();

        let mut zero_duration_item_paths = vec![];

        let mut rss_items: Vec<RSSItem> = match playlist.entries {
            Some(ref entries) => entries
                .iter()
                .map(|video| {
                    use hhmmss::Hhmmss;

                    let duration = match &video.duration {
                        Some(value) => {
                            let secs = match value {
                                serde_json::Value::Number(secs) => secs.as_f64().unwrap_or(0.0),
                                _ => 0.0,
                            };
                            Duration::new(secs as u64, 0)
                        }
                        None => Duration::default(),
                    };

                    let item_path = Path::new(&self.feed_file.parent().unwrap())
                        .join(self.feed_file.file_stem().unwrap())
                        .join(format!("{}.mp4", video.id));

                    if duration.is_zero() {
                        zero_duration_item_paths.push(item_path);
                    }

                    let item_itunes_extension = ITunesItemExtensionBuilder::default()
                        .author(title.clone())
                        .subtitle(video.title.clone())
                        .summary(video.description.clone())
                        .image(video.thumbnail.clone())
                        .duration(duration.hhmmss())
                        .explicit("No".to_string())
                        .build();

                    let item_enclosure = RSSEnclosureBuilder::default()
                        .url(
                            base_url
                                .join(&format!(
                                    "{}/",
                                    self.feed_file.file_stem().unwrap().to_string_lossy()
                                ))
                                .unwrap()
                                .join(&format!("{}.mp4", video.id))
                                .unwrap(),
                        )
                        .length(
                            (video
                                .filesize
                                .unwrap_or_else(|| video.filesize_approx.unwrap_or(0.0) as i64))
                            .to_string(),
                        )
                        .mime_type("video/mp4")
                        .build();

                    // video.release_date
                    // video.upload_date

                    let mut item = RSSItemBuilder::default();

                    item.guid(RSSGuidBuilder::default().value(video.id.clone()).build())
                        .title(video.title.clone())
                        .description(video.description.clone())
                        .link(video.webpage_url.clone())
                        .enclosure(item_enclosure)
                        .itunes_ext(item_itunes_extension);

                    if let Some(upload_date) = &video.upload_date {
                        item.pub_date(
                            Utc.from_utc_datetime(
                                &NaiveDateTime::parse_from_str(
                                    &format!("{}T00:00Z", upload_date),
                                    "%Y%m%dT%H:%MZ",
                                )
                                .unwrap_or_else(|error| {
                                    panic!("Error: {} parsing {:?}", error, upload_date)
                                }),
                            )
                            .to_rfc2822(),
                        );
                    } else if let Some(release_date) = &video.release_date {
                        item.pub_date(
                            Utc.from_utc_datetime(
                                &NaiveDateTime::parse_from_str(
                                    &format!("{}T00:00Z", release_date),
                                    "%Y%m%dT%H:%MZ",
                                )
                                .unwrap_or_else(|error| {
                                    panic!("Error: {} parsing {:?}", error, release_date)
                                }),
                            )
                            .to_rfc2822(),
                        );
                    }

                    item.build()
                })
                .collect(),
            None => vec![],
        };

        if !rss_items.is_empty() && zero_duration_item_paths.len() == rss_items.len() {
            return Err(Error::AllDownloadsEmptyError(self.playlist_url.clone()));
        } else if !zero_duration_item_paths.is_empty() {
            warn!("One or more files were not found on disk!\nYour playlist URL might be invalid. {:?}", zero_duration_item_paths);
        }

        // Retrieve the existing RSS channel, or create a new one
        let mut rss_channel = self.rss_channel.clone().unwrap_or_else(|| {
            let description = format!("{} podcast feed for {}", PKG_NAME, title);

            let rss_itunes_category = ITunesCategoryBuilder::default().text("TV & Film").build();

            let rss_itunes_extension = ITunesChannelExtensionBuilder::default()
                .author(title.clone())
                .subtitle(title.clone())
                .summary(description.clone())
                .explicit("No".to_string())
                .category(rss_itunes_category)
                .block("Yes".to_string())
                .build();

            RSSChannelBuilder::default()
                .title(title)
                .description(description)
                .itunes_ext(rss_itunes_extension)
                .build()
        });

        rss_items.append(&mut rss_channel.items);

        let mut unique_items: Vec<_> = rss_items
            .into_iter()
            .unique_by(|item| item.guid().unwrap().value().to_string())
            .collect();

        if let Some(keep_item_count) = keep {
            if unique_items.len() > keep_item_count {
                let removed_items: Vec<_> = unique_items.drain(keep_item_count..).collect();

                for item in removed_items {
                    let id = item.guid().unwrap().value().to_string();

                    let path = Path::new(
                        &self
                            .feed_file
                            .parent()
                            .ok_or_else(|| Error::ParentPathError(self.feed_file.clone()))?,
                    )
                    .join(
                        self.feed_file
                            .file_stem()
                            .ok_or_else(|| Error::FileStemError(self.feed_file.clone()))?,
                    )
                    .join(format!("{}.mp4", id));

                    debug!("Attempting to remove file: {:?}", path);

                    std::fs::remove_file(path)
                        .unwrap_or_else(|err| warn!("Couldn't remove file: {:?}", err));
                }
            }
        }

        if let Some(ref mut channel_itunes_ext) = rss_channel.itunes_ext {
            for item in &unique_items {
                if let Some(ref item_ext) = item.itunes_ext {
                    channel_itunes_ext.image.clone_from(&item_ext.image);
                    break;
                }
            }
        }

        rss_channel.set_link(
            playlist
                .webpage_url
                .unwrap_or_else(|| self.playlist_url.to_string()),
        );
        rss_channel.set_generator(format!("{}/{} ({})", PKG_NAME, PKG_VERSION, PKG_HOMEPAGE));
        rss_channel.set_items(unique_items);

        self.rss_channel = Some(rss_channel);

        Ok(())
    }

    pub fn update(&mut self, base_url: Url, keep: Option<usize>) -> Result<(), Error> {
        self.update_with_args(base_url, DEFAULT_DOWNLOAD_LIMIT, keep, vec![])
    }

    pub fn update_with_args(
        &mut self,
        base_url: Url,
        download_limit: usize,
        keep: Option<usize>,
        additional_args: Vec<String>,
    ) -> Result<(), Error> {
        let mut ytdl = YoutubeDl::new(self.playlist_url.clone());

        ytdl.youtube_dl_path("yt-dlp");

        ytdl.extra_arg("--playlist-end")
            .extra_arg(download_limit.to_string());

        ytdl.extra_arg("--format")
            .extra_arg("bestvideo[ext=mp4][vcodec^=avc1]+bestaudio[ext=m4a]/best[ext=mp4][vcodec^=avc1]/best[ext=mp4]/best");

        ytdl.extra_arg("--no-simulate");

        additional_args.into_iter().for_each(|arg| {
            ytdl.extra_arg(arg);
        });

        // NOTE: Required because `yt-dlp` prints progress to stdout and breaks YoutubeDl when `--no-simulate` is specified
        ytdl.extra_arg("--no-progress");
        ytdl.extra_arg("--no-overwrites");
        ytdl.extra_arg("--output").extra_arg(
            Path::new(
                &self
                    .feed_file
                    .parent()
                    .ok_or_else(|| Error::ParentPathError(self.feed_file.clone()))?,
            )
            .join(
                self.feed_file
                    .file_stem()
                    .ok_or_else(|| Error::FileStemError(self.feed_file.clone()))?,
            )
            .join("%(id)s.%(ext)s")
            .to_string_lossy(),
        );

        let result = ytdl.run()?;

        trace!("{:#?}", result);

        if let YoutubeDlOutput::Playlist(playlist) = result {
            self.update_with_playlist(base_url, keep, *playlist)
        } else {
            panic!("This URL points to a single video, not a channel!")
        }
    }
}

#[cfg(test)]
mod test {
    fn get_new_video() -> youtube_dl::SingleVideo {
        youtube_dl::SingleVideo {
            abr: Some(129.478),
            acodec: Some("mp4a.40.2".to_string()),
            age_limit: Some(0),
            album: None,
            album_artist: None,
            album_type: None,
            alt_title: None,
            artist: None,
            asr: Some(44100.0),
            automatic_captions: None,
            average_rating: None,
            categories: None,
            channel: Some("Mighty Car Mods".to_string()),
            channel_id: Some("UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            channel_url: Some("https://www.youtube.com/channel/UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            chapter: None,
            chapter_id: None,
            chapter_number: None,
            chapters: None,
            comment_count: None,
            comments: None,
            container: None,
            creator: None,
            description: Some("Do you wanna make your car better, more enjoyable for car meets, off-roading, road trips or just day to day commuting? Then this simple modification is for you! \n\nAs the best 4X4 channel in the world, we'll show you exactly what you need to do to get the job done! \n\nYou can use this additional power to run a fridge, audio equipment, charging systems or even a travel oven. \n\nMERCH SHOWN IN THIS EPISODE\n\n►MCM Travel Mug\nhttps://mightycarmods.com/collections/accessories/products/travel-mug\n\n►Enamel Workshop Mug\nhttps://mightycarmods.com/collections/accessories/products/enamel-workshop-mug\n\n►Chopped Ceramic Mug\nhttps://mightycarmods.com/collections/accessories/products/chopped-mug\n\n►Insulated Drink Bottle\nhttps://mightycarmods.com/collections/accessories/products/mcm-drink-bottle\n\n►Fender Covers\nhttps://mightycarmods.com/collections/accessories/products/fender-cover-2-pack\n\n►Microfibre Cloths\nhttps://mightycarmods.com/collections/accessories/products/mighty-car-mods-microfibre-cloth-4-pack\n\n►MCM Ear Muffs\nhttps://mightycarmods.com/collections/accessories/products/ear-muffs\n\n►MCM BOOK [ULTIMATE EDITION] Autographed\nhttps://mightycarmods.com/collections/books/products/copy-of-the-cars-of-mighty-car-mods-ultimate-edition-hardcover-book\n\nMORE MERCH HERE ►http://www.mightycarmods.com/collections/\n\nBe sure to check out the MCM Facebook Page for regular Updates\n►http://www.facebook.com/mightycarmods\n\n✌\u{fe0f} JDM Air Fresheners https://mightycarmods.com/collections/accessories/products/mcm-car-air-freshener \n👕 BRAND NEW EXPLODED TURBO SHIRT\nhttps://mightycarmods.com/collections/clothing/products/exploded-turbo-shirt\nLIMITED EDITION MCM BOOK (SIGNED)\nhttps://mightycarmods.com/collections/books/products/copy-of-the-cars-of-mighty-car-mods-ultimate-edition-hardcover-book\n\n🔔 Hit the bell next to Subscribe so you don't miss a video!\n\nAlso something to note around Mighty Car Mods: we are normal guys and are not trained mechanics. We like to make interesting car mods and show you how we've gone about it, but we can't promise that anything we show you will work for your particular car, or that you won't harm yourself, someone else, your car or your warranty doing it. Please be safe, be responsible and unless you know what you're doing, do not fool around with very serious machinery just because you've seen us make it look so easy. Talk to a qualified mechanic if you are in any doubt. Some of the products featured in this video may be supplied by sponsors. For a list of our current sponsors please go to mightycarmods.com".to_string()),
            disc_number: None,
            dislike_count: None,
            display_id: Some("QWkUFkXcx9I".to_string()),
            downloader_options: None,
            duration: Some(serde_json::Value::Number(serde_json::Number::from_f64(706.0).unwrap())),
            end_time: None,
            episode: None,
            episode_id: None,
            episode_number: None,
            ext: Some("mp4".to_string()),
            extractor: Some("youtube".to_string()),
            extractor_key: Some("Youtube".to_string()),
            filesize: None,
            filesize_approx: Some(212973334.0),
            format: Some("137 - 1920x1080 (1080p)+140 - audio only (medium)".to_string()),
            format_id: Some("137+140".to_string()),
            format_note: Some("1080p+medium".to_string()),
            formats: None,
            fps: Some(25.0),
            fragment_base_url: None,
            fragments: None,
            genre: None,
            height: Some(1080.0),
            http_headers: None,
            id: "QWkUFkXcx9I".to_string(),
            is_live: Some(false),
            language: None,
            language_preference: None,
            license: None,
            like_count: Some(16399),
            location: None,
            manifest_url: None,
            no_resume: None,
            player_url: None,
            playlist: Some("Mighty Car Mods - Videos".to_string()),
            playlist_id: Some("UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            playlist_index: Some(serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap())),
            playlist_title: Some("Mighty Car Mods - Videos".to_string()),
            playlist_uploader: Some("Mighty Car Mods".to_string()),
            playlist_uploader_id: Some("UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            preference: None,
            protocol: Some(youtube_dl::Protocol::HttpsHttps),
            quality: None,
            release_date: None,
            release_year: None,
            repost_count: None,
            requested_subtitles: None,
            resolution: Some("1920x1080".to_string()),
            season: None,
            season_id: None,
            season_number: None,
            series: None,
            source_preference: None,
            start_time: None,
            stretched_ratio: None,
            subtitles: None,
            tags: None,
            tbr: Some(2411.782),
            thumbnail: Some("https://i.ytimg.com/vi/QWkUFkXcx9I/maxresdefault.jpg".to_string()),
            thumbnails: None,
            timestamp: None,
            title: Some("Everyone Should do this Simple $10 Car Mod".to_string()),
            track: None,
            track_id: None,
            track_number: None,
            upload_date: Some("20220206".to_string()),
            uploader: Some("Mighty Car Mods".to_string()),
            uploader_id: Some("mightycarmods".to_string()),
            uploader_url: Some("http://www.youtube.com/user/mightycarmods".to_string()),
            url: None,
            vbr: Some(2282.304),
            vcodec: Some("avc1.640028".to_string()),
            view_count: Some(294645),
            webpage_url: Some("https://www.youtube.com/watch?v=QWkUFkXcx9I".to_string()),
            width: Some(1920.0),
            ..Default::default()
        }
    }

    fn get_duplicate_video() -> youtube_dl::SingleVideo {
        youtube_dl::SingleVideo {
            abr: Some(129.478),
            acodec: Some("mp4a.40.2".to_string()),
            age_limit: Some(0),
            album: None,
            album_artist: None,
            album_type: None,
            alt_title: None,
            artist: None,
            asr: Some(44100.0),
            automatic_captions: None,
            average_rating: None,
            categories: None,
            channel: Some("Mighty Car Mods".to_string()),
            channel_id: Some("UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            channel_url: Some("https://www.youtube.com/channel/UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            chapter: None,
            chapter_id: None,
            chapter_number: None,
            chapters: None,
            comment_count: None,
            comments: None,
            container: None,
            creator: None,
            description: Some("Do you wanna make your car better, more enjoyable for car meets, off-roading, road trips or just day to day commuting? Then this simple modification is for you! \n\nAs the best 4X4 channel in the world, we'll show you exactly what you need to do to get the job done! \n\nYou can use this additional power to run a fridge, audio equipment, charging systems or even a travel oven. \n\nMERCH SHOWN IN THIS EPISODE\n\n►MCM Travel Mug\nhttps://mightycarmods.com/collections/accessories/products/travel-mug\n\n►Enamel Workshop Mug\nhttps://mightycarmods.com/collections/accessories/products/enamel-workshop-mug\n\n►Chopped Ceramic Mug\nhttps://mightycarmods.com/collections/accessories/products/chopped-mug\n\n►Insulated Drink Bottle\nhttps://mightycarmods.com/collections/accessories/products/mcm-drink-bottle\n\n►Fender Covers\nhttps://mightycarmods.com/collections/accessories/products/fender-cover-2-pack\n\n►Microfibre Cloths\nhttps://mightycarmods.com/collections/accessories/products/mighty-car-mods-microfibre-cloth-4-pack\n\n►MCM Ear Muffs\nhttps://mightycarmods.com/collections/accessories/products/ear-muffs\n\n►MCM BOOK [ULTIMATE EDITION] Autographed\nhttps://mightycarmods.com/collections/books/products/copy-of-the-cars-of-mighty-car-mods-ultimate-edition-hardcover-book\n\nMORE MERCH HERE ►http://www.mightycarmods.com/collections/\n\nBe sure to check out the MCM Facebook Page for regular Updates\n►http://www.facebook.com/mightycarmods\n\n✌\u{fe0f} JDM Air Fresheners https://mightycarmods.com/collections/accessories/products/mcm-car-air-freshener \n👕 BRAND NEW EXPLODED TURBO SHIRT\nhttps://mightycarmods.com/collections/clothing/products/exploded-turbo-shirt\nLIMITED EDITION MCM BOOK (SIGNED)\nhttps://mightycarmods.com/collections/books/products/copy-of-the-cars-of-mighty-car-mods-ultimate-edition-hardcover-book\n\n🔔 Hit the bell next to Subscribe so you don't miss a video!\n\nAlso something to note around Mighty Car Mods: we are normal guys and are not trained mechanics. We like to make interesting car mods and show you how we've gone about it, but we can't promise that anything we show you will work for your particular car, or that you won't harm yourself, someone else, your car or your warranty doing it. Please be safe, be responsible and unless you know what you're doing, do not fool around with very serious machinery just because you've seen us make it look so easy. Talk to a qualified mechanic if you are in any doubt. Some of the products featured in this video may be supplied by sponsors. For a list of our current sponsors please go to mightycarmods.com".to_string()),
            disc_number: None,
            dislike_count: None,
            display_id: Some("Wqww1B9wljA".to_string()),
            downloader_options: None,
            duration: Some(serde_json::Value::Number(serde_json::Number::from_f64(706.0).unwrap())),
            end_time: None,
            episode: None,
            episode_id: None,
            episode_number: None,
            ext: Some("mp4".to_string()),
            extractor: Some("youtube".to_string()),
            extractor_key: Some("Youtube".to_string()),
            filesize: None,
            filesize_approx: Some(212973334.0),
            format: Some("137 - 1920x1080 (1080p)+140 - audio only (medium)".to_string()),
            format_id: Some("137+140".to_string()),
            format_note: Some("1080p+medium".to_string()),
            formats: None,
            fps: Some(25.0),
            fragment_base_url: None,
            fragments: None,
            genre: None,
            height: Some(1080.0),
            http_headers: None,
            id: "QWkUFkXcx9I".to_string(),
            is_live: Some(false),
            language: None,
            language_preference: None,
            license: None,
            like_count: Some(16399),
            location: None,
            manifest_url: None,
            no_resume: None,
            player_url: None,
            playlist: Some("Mighty Car Mods - Videos".to_string()),
            playlist_id: Some("UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            playlist_index: Some(serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap())),
            playlist_title: Some("Mighty Car Mods - Videos".to_string()),
            playlist_uploader: Some("Mighty Car Mods".to_string()),
            playlist_uploader_id: Some("UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            preference: None,
            protocol: Some(youtube_dl::Protocol::HttpsHttps),
            quality: None,
            release_date: None,
            release_year: None,
            repost_count: None,
            requested_subtitles: None,
            resolution: Some("1920x1080".to_string()),
            season: None,
            season_id: None,
            season_number: None,
            series: None,
            source_preference: None,
            start_time: None,
            stretched_ratio: None,
            subtitles: None,
            tags: None,
            tbr: Some(2411.782),
            thumbnail: Some("https://i.ytimg.com/vi/QWkUFkXcx9I/maxresdefault.jpg".to_string()),
            thumbnails: None,
            timestamp: None,
            title: Some("Everyone Should do this Simple $10 Car Mod".to_string()),
            track: None,
            track_id: None,
            track_number: None,
            upload_date: Some("20220206".to_string()),
            uploader: Some("Mighty Car Mods".to_string()),
            uploader_id: Some("mightycarmods".to_string()),
            uploader_url: Some("http://www.youtube.com/user/mightycarmods".to_string()),
            url: None,
            vbr: Some(2282.304),
            vcodec: Some("avc1.640028".to_string()),
            view_count: Some(294645),
            webpage_url: Some("https://www.youtube.com/watch?v=QWkUFkXcx9I".to_string()),
            width: Some(1920.0),
            ..Default::default()
        }
    }

    use crate::Error;

    #[test]
    fn test_update_new_with_playlist() -> Result<(), Error> {
        use rss::validation::Validate;
        use url::Url;

        let playlist = youtube_dl::model::Playlist {
            entries: Some(vec![get_new_video()]),
            extractor: Some("youtube:tab".to_string()),
            extractor_key: Some("YoutubeTab".to_string()),
            id: Some("UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            title: Some("Mighty Car Mods - Videos".to_string()),
            uploader: Some("Mighty Car Mods".to_string()),
            uploader_id: Some("UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            uploader_url: Some(
                "https://www.youtube.com/channel/UCgJRL30YS6XFxq9Ga8W2J3A".to_string(),
            ),
            webpage_url: Some("https://www.youtube.com/c/mightycarmods".to_string()),
            webpage_url_basename: Some("mightycarmods".to_string()),
            ..Default::default()
        };

        let mut channel = super::Channel::new_with_url(
            std::path::Path::new("mightycarmods.xml").to_path_buf(),
            Url::parse("https://www.youtube.com/c/mightycarmods").unwrap(),
        )?;

        channel.update_with_playlist(Url::parse("http://localhost").unwrap(), None, playlist)?;
        let rss_channel = channel.rss_channel.unwrap();
        rss_channel.validate().unwrap();

        assert_eq!(rss_channel.items.len(), 1);

        Ok(())
    }

    #[test]
    fn test_update_existing_with_playlist_and_truncation() -> Result<(), Error> {
        use rss::validation::Validate;
        use std::io::BufReader;
        use url::Url;

        let playlist = youtube_dl::model::Playlist {
            entries: Some(vec![get_new_video(), get_duplicate_video()]),
            extractor: Some("youtube:tab".to_string()),
            extractor_key: Some("YoutubeTab".to_string()),
            id: Some("UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            title: Some("Mighty Car Mods - Videos".to_string()),
            uploader: Some("Mighty Car Mods".to_string()),
            uploader_id: Some("UCgJRL30YS6XFxq9Ga8W2J3A".to_string()),
            uploader_url: Some(
                "https://www.youtube.com/channel/UCgJRL30YS6XFxq9Ga8W2J3A".to_string(),
            ),
            webpage_url: Some("https://www.youtube.com/c/mightycarmods".to_string()),
            webpage_url_basename: Some("mightycarmods".to_string()),
            ..Default::default()
        };

        let bytes = include_bytes!("../fixtures/mightycarmods.rss");
        let reader = BufReader::new(&bytes[0..]);

        let mut channel = super::Channel::new_with_reader(
            std::path::Path::new("mightycarmods.xml").to_path_buf(),
            reader,
        )?;

        let rss_channel = channel.rss_channel.as_ref().unwrap();
        assert_eq!(rss_channel.items.len(), 1);

        channel.update_with_playlist(
            Url::parse("http://localhost:8080").unwrap(),
            None,
            playlist.clone(),
        )?;
        let rss_channel = channel.rss_channel.as_ref().unwrap();
        rss_channel.validate().unwrap();

        assert_eq!(rss_channel.items.len(), 2);

        assert_eq!(
            rss_channel.items[0].enclosure.as_ref().unwrap().url,
            "http://localhost:8080/mightycarmods/QWkUFkXcx9I.mp4"
        );
        assert_eq!(
            rss_channel.items[1].enclosure.as_ref().unwrap().url,
            "http://localhost:8080/mightycarmods/Wqww1B9wljA.mp4"
        );

        channel.update_with_playlist(
            Url::parse("http://localhost:8080").unwrap(),
            Some(1),
            playlist.clone(),
        )?;
        let rss_channel = channel.rss_channel.unwrap();
        rss_channel.validate().unwrap();

        assert_eq!(rss_channel.items.len(), 1);

        Ok(())
    }
}
