# Playcaster

[![crates.io](https://img.shields.io/crates/v/playcaster.svg)](https://crates.io/crates/playcaster) [![Rust](https://github.com/ticky/playcaster/actions/workflows/rust.yml/badge.svg)](https://github.com/ticky/playcaster/actions/workflows/rust.yml) [![Docker](https://github.com/ticky/playcaster/actions/workflows/docker-publish.yml/badge.svg)](https://github.com/ticky/playcaster/actions/workflows/docker-publish.yml)

Turn any playlist[^1] into a Podcast feed

## Installation

Playcaster can be installed from Cargo:

```sh
cargo install playcaster
```

You will additionally need `yt-dlp` installed. Instructions for that can be found at <https://github.com/yt-dlp/yt-dlp#installation>.

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

NOTE: Since [yt-dlp 2022.11.11](https://github.com/yt-dlp/yt-dlp/releases/tag/2022.11.11), plain YouTube channel URLs download as a series of playlists. Playcaster v0.0.2 has been updated to emit an error if all of the items in the target playlist have an apparent duration of zero. You may need to update channel URLs to refer to a specific tab (i.e. `/videos`) or use a playlist instead.

Items after `--` are passed on to `yt-dlp`, to configure its extraction or filter results.

## Docker Installation & Usage

A Docker image is supplied for ease of use in environments like a NAS, and can be installed with the following command:

```sh
docker pull ghcr.io/ticky/playcaster:main
```

It can be run as follows, substituting `$HOME/htdocs/feeds` for where your feeds should be on your host system:

```sh
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
