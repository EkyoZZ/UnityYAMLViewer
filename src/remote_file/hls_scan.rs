//! Bounded static-source discovery only; no page JavaScript or network hooks.
pub(super) fn scan(text: &str, page: &str) -> Vec<String> {
    let Ok(base) = reqwest::Url::parse(page) else { return Vec::new(); };
    let decoded = super::decode_pasted_unicode(text).replace("\\/", "/");
    let pattern = regex::Regex::new(r#"(?i)(?:https?://|//|/|\./|\.\./)[^\s"'<>\\]*?\.m3u8(?:\?[^\s"'<>\\]*)?|["']([^\s"'<>\\]+?\.m3u8(?:\?[^\s"'<>\\]*)?)["']"#).unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut found = Vec::new();
    for capture in pattern.captures_iter(&decoded) {
        let raw = capture.get(1).or_else(|| capture.get(0)).unwrap().as_str();
        let unescaped = quick_xml::escape::unescape(raw).unwrap_or_else(|_| raw.into());
        let Ok(mut url) = base.join(&unescaped) else { continue; };
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() { continue; }
        url.set_fragment(None);
        if seen.insert(url.as_str().to_owned()) {
            found.push(url.into());
            if found.len() == 300 { break; }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_and_deduplicates_static_sources_without_decoding_percent_encoding() {
        let html = r#"<source src="/a.m3u8?a=1&amp;b=%26%2B"><script>let x="https:\/\/cdn.example.org/b.m3u8?a=1\u0026b=2";</script><source src="/a.m3u8?a=1&amp;b=%26%2B"><source src="relative.m3u8">"#;
        assert_eq!(scan(html, "https://example.org/watch/"), [
            "https://example.org/a.m3u8?a=1&b=%26%2B",
            "https://cdn.example.org/b.m3u8?a=1&b=2",
            "https://example.org/watch/relative.m3u8",
        ]);
    }
    #[test]
    fn ignores_pages_without_candidates_and_limits_the_list() {
        assert!(scan("<video></video>", "https://example.org/").is_empty());
        let html = (0..400).map(|i| format!("\"https://example.org/{i}.m3u8\" ")).collect::<String>();
        assert_eq!(scan(&html, "https://example.org/").len(), 300);
    }
}
