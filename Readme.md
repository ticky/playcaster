# Playcaster

[![Rust](https://github.com/ticky/playcaster/actions/workflows/rust.yml/badge.svg)](https://github.com/ticky/playcaster/actions/workflows/rust.yml) [![Docker](https://github.com/ticky/playcaster/actions/workflows/docker-publish.yml/badge.svg)](https://github.com/ticky/playcaster/actions/workflows/docker-publish.yml)

Turn any playlist[^1] into a Podcast feed

## Usage

`playcaster <channel-path> <url> <hostname> [downloader-arguments]...`

```sh
playcaster \
	htdocs/playlist \
	"https://www.youtube.com/playlist?list=playlist" \
	"http://your-podcast-server.example/playlist" \
	--write-feed \
	-- \
		--embed-chapters \
		--write-auto-sub \
		--embed-subs \
		--sub-lang en
```

`--write-feed` tells playcaster it should write an RSS feed to a file adjacent to `channel-path`, otherwise it writes the feed XML to the terminal.

Items after `--` are passed on to `yt-dlp`, to configure its extraction or filter results.

[^1]: That `yt-dlp` supports, anyway
