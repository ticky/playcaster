# Playcaster

[![crates.io](https://img.shields.io/crates/v/playcaster.svg)](https://crates.io/crates/playcaster) [![Rust](https://github.com/ticky/playcaster/actions/workflows/rust.yml/badge.svg)](https://github.com/ticky/playcaster/actions/workflows/rust.yml) [![Docker](https://github.com/ticky/playcaster/actions/workflows/docker-publish.yml/badge.svg)](https://github.com/ticky/playcaster/actions/workflows/docker-publish.yml)

Turn any playlist[^1] into a Podcast feed

## Usage

`playcaster <feed-file> <base-url> [downloader-arguments]...`

```sh
playcaster \
	$HOME/htdocs/feeds/playlist.xml \
	"http://your-podcast-server.example" \
	--playlist-url "https://www.youtube.com/playlist?list=playlist" \
	-- \
		--embed-chapters \
		--write-auto-sub \
		--embed-subs \
		--sub-lang en
```

`--playlist-url` specifies the playlist to fetch items from. It only needs to be specified if `<feed-file>` doesn't exist yet, or doesn't have a `<link/>` which already points to the playlist.

Items after `--` are passed on to `yt-dlp`, to configure its extraction or filter results.

## Docker Usage

A Docker image is supplied for ease of use in environments like a NAS:

```sh
docker pull ghcr.io/ticky/playcaster:main
docker run --rm -v $HOME/htdocs/feeds:/feeds -it ghcr.io/ticky/playcaster:main \
		/feeds/playlist.xml \
		"http://your-podcast-server.example" \
		--playlist-url "https://www.youtube.com/playlist?list=playlist" \
		-- \
			--embed-chapters \
			--write-auto-sub \
			--embed-subs \
			--sub-lang en
```

The image is based upon [jauderho/yt-dlp](https://hub.docker.com/r/jauderho/yt-dlp), which includes `yt-dlp` and `ffmpeg`.

[^1]: That `yt-dlp` supports, anyway
