# hls-proxy

A lightweight HTTP reverse proxy for HLS streams. Given any HLS URL, it fetches and rewrites the playlist so that all segment and child-playlist URLs are routed back through the proxy — letting clients play streams that would otherwise be blocked by CORS or network restrictions.

Both master playlists (multi-bitrate) and media playlists (segments) are handled transparently. Segment bytes are streamed straight through without buffering.

## Docker

```sh
docker run -d \
  -p 8080:8080 \
  -e BASE_URL=https://your-proxy-domain.com \
  ghcr.io/fanchao/hls-proxy:latest
```

Then play any HLS stream through the proxy:

```
https://your-proxy-domain.com/proxy?url=https://upstream.example.com/stream.m3u8
```

## Environment variables

| Variable           | CLI flag             | Default         | Description                                                                 |
|--------------------|----------------------|-----------------|-----------------------------------------------------------------------------|
| `BASE_URL`         | `--base-url`         | *(required)*    | Public-facing URL of this proxy. Embedded into rewritten playlist URLs.     |
| `BIND`             | `--bind`             | `0.0.0.0:8080`  | Address and port to listen on.                                              |
| `UPSTREAM_TIMEOUT` | `--upstream-timeout` | `30`            | Timeout in seconds for upstream requests.                                   |
| `LOG_LEVEL`        | `--log-level`        | `info`          | Log level: `trace`, `debug`, `info`, `warn`, or `error`.                   |
