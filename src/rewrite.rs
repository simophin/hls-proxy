use url::Url;

/// Rewrites all URLs in an m3u8 playlist body so that they point back through
/// the proxy at `proxy_base` with the original resolved URL as the `url` query
/// param.
///
/// `playlist_url` is the URL the playlist was fetched from; it is used to
/// resolve relative segment/playlist references before encoding them.
pub fn rewrite_playlist(body: &str, playlist_url: &Url, proxy_base: &str) -> String {
    let proxy_base = proxy_base.trim_end_matches('/');
    let mut out = String::with_capacity(body.len());

    for line in body.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("#EXT-X-KEY") {
            out.push_str(&rewrite_ext_x_key(line, playlist_url, proxy_base));
        } else if trimmed.starts_with('#') || trimmed.is_empty() {
            out.push_str(line);
        } else {
            // URI line: either a segment or a child playlist
            out.push_str(&proxied_url(trimmed, playlist_url, proxy_base));
        }

        out.push('\n');
    }

    out
}

/// Rewrites the URI= attribute inside an #EXT-X-KEY tag.
fn rewrite_ext_x_key(line: &str, playlist_url: &Url, proxy_base: &str) -> String {
    // Replace URI="<value>" with a proxied version.
    let Some(uri_start) = line.find("URI=\"") else {
        return line.to_string();
    };
    let value_start = uri_start + 5; // after `URI="`
    let Some(value_end) = line[value_start..].find('"') else {
        return line.to_string();
    };
    let value_end = value_start + value_end;

    let original_uri = &line[value_start..value_end];
    let proxied = proxied_url(original_uri, playlist_url, proxy_base);

    format!("{}URI=\"{}\"{}", &line[..uri_start], proxied, &line[value_end + 1..])
}

/// Resolves `raw` relative to `base_url` and returns a proxy URL for it.
fn proxied_url(raw: &str, base_url: &Url, proxy_base: &str) -> String {
    let resolved = match base_url.join(raw) {
        Ok(u) => u,
        Err(_) => return raw.to_string(),
    };
    format!("{}/proxy?url={}", proxy_base, urlencoding::encode(resolved.as_str()))
}

// ------------------------------------------------------------------
// urlencoding shim — keeps the dependency count low
// ------------------------------------------------------------------
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'-'
                | b'_'
                | b'.'
                | b'~' => out.push(byte as char),
                b => out.push_str(&format!("%{:02X}", b)),
            }
        }
        out
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    fn base() -> Url {
        Url::parse("https://upstream.example.com/hls/stream.m3u8").unwrap()
    }

    fn proxy() -> &'static str {
        "https://proxy.example.com"
    }

    // ---- proxied_url ----

    #[test]
    fn proxied_url_absolute() {
        let result = proxied_url(
            "https://cdn.example.com/seg/0001.ts",
            &base(),
            proxy(),
        );
        assert_eq!(
            result,
            "https://proxy.example.com/proxy?url=https%3A%2F%2Fcdn.example.com%2Fseg%2F0001.ts"
        );
    }

    #[test]
    fn proxied_url_relative_same_dir() {
        let result = proxied_url("0001.ts", &base(), proxy());
        assert_eq!(
            result,
            "https://proxy.example.com/proxy?url=https%3A%2F%2Fupstream.example.com%2Fhls%2F0001.ts"
        );
    }

    #[test]
    fn proxied_url_relative_with_path() {
        let result = proxied_url("../segments/0001.ts", &base(), proxy());
        assert_eq!(
            result,
            "https://proxy.example.com/proxy?url=https%3A%2F%2Fupstream.example.com%2Fsegments%2F0001.ts"
        );
    }

    // ---- rewrite_playlist ----

    #[test]
    fn rewrites_segment_uris() {
        let m3u8 = "#EXTM3U\n#EXT-X-VERSION:3\n#EXTINF:6.0,\nseg-0001.ts\n#EXT-INF:6.0,\nseg-0002.ts\n#EXT-X-ENDLIST\n";
        let out = rewrite_playlist(m3u8, &base(), proxy());
        assert!(out.contains("https://proxy.example.com/proxy?url=https%3A%2F%2Fupstream.example.com%2Fhls%2Fseg-0001.ts"));
        assert!(out.contains("https://proxy.example.com/proxy?url=https%3A%2F%2Fupstream.example.com%2Fhls%2Fseg-0002.ts"));
        // Comments and tags left untouched
        assert!(out.contains("#EXTM3U"));
        assert!(out.contains("#EXT-X-VERSION:3"));
        assert!(out.contains("#EXT-X-ENDLIST"));
    }

    #[test]
    fn rewrites_master_playlist_variant_uris() {
        let m3u8 = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000\nlow/stream.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2000000\nhigh/stream.m3u8\n";
        let out = rewrite_playlist(m3u8, &base(), proxy());
        assert!(out.contains("https://proxy.example.com/proxy?url=https%3A%2F%2Fupstream.example.com%2Fhls%2Flow%2Fstream.m3u8"));
        assert!(out.contains("https://proxy.example.com/proxy?url=https%3A%2F%2Fupstream.example.com%2Fhls%2Fhigh%2Fstream.m3u8"));
        assert!(out.contains("#EXT-X-STREAM-INF:BANDWIDTH=800000"));
    }

    #[test]
    fn preserves_empty_lines() {
        let m3u8 = "#EXTM3U\n\nseg.ts\n";
        let out = rewrite_playlist(m3u8, &base(), proxy());
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "#EXTM3U");
        assert_eq!(lines[1], "");
        assert!(lines[2].starts_with("https://proxy.example.com"));
    }

    #[test]
    fn rewrites_ext_x_key_uri() {
        let m3u8 = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"https://keys.example.com/key\",IV=0x0\nseg.ts\n";
        let out = rewrite_playlist(m3u8, &base(), proxy());
        assert!(out.contains("URI=\"https://proxy.example.com/proxy?url=https%3A%2F%2Fkeys.example.com%2Fkey\""));
        assert!(out.contains("METHOD=AES-128"));
        assert!(out.contains("IV=0x0"));
    }

    #[test]
    fn ext_x_key_without_uri_left_unchanged() {
        let line = "#EXT-X-KEY:METHOD=NONE";
        let out = rewrite_playlist(line, &base(), proxy());
        assert!(out.trim() == line);
    }

    // ---- urlencoding shim ----

    #[test]
    fn url_encoding_round_trip() {
        let s = "https://example.com/path?foo=bar&baz=qux";
        let encoded = super::urlencoding::encode(s);
        assert!(!encoded.contains('?'));
        assert!(!encoded.contains('&'));
        assert!(!encoded.contains('='));
        assert!(encoded.contains("%3F") || encoded.contains("%3f"));
    }
}
