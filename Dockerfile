FROM rust:alpine as builder

WORKDIR /app

RUN set -x && \
	apk add --no-cache \
		musl-dev \
		pkgconfig \
		openssl \
		openssl-dev

# Create the skeleton of a Rust app
RUN cargo init

# Bring in our real-world dependencies
COPY Cargo.toml Cargo.lock .

# Install our dependencies so they can be cached
RUN set -x && \
	cargo build && \
	cargo clean -p playcaster

# Then copy in our actual source code and build the real thing
COPY src src

RUN cargo install --path .

# Now move over to our yt-dlp container
FROM jauderho/yt-dlp:2023.12.30

# Copy in our freshly-baked binaries
COPY --from=builder /usr/local/cargo/bin/* /usr/local/bin

WORKDIR /feeds
VOLUME ["/feeds"]

# And let's go!
RUN dumb-init playcaster --version

ENTRYPOINT ["dumb-init", "playcaster"]
CMD ["--help"]
