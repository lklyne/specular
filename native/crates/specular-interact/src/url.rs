//! Telling a URL from other text, and completing one a person typed.
//! Ported from `src/shared/url.ts`.

/// Whether `value` is a URL a page can show: it has an `http` or `https`
/// scheme, or it reads as a bare host (`example.com/path`, `localhost:3000`).
/// Text with whitespace in it, and any other scheme, is not.
pub fn looks_like_url(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains(char::is_whitespace) {
        return false;
    }
    if let Some(rest) = strip_prefix_ignoring_case(trimmed, "http://")
        .or_else(|| strip_prefix_ignoring_case(trimmed, "https://"))
    {
        return !authority(rest).is_empty();
    }
    let host = authority(trimmed);
    if is_local_host_with_port(host) {
        return true;
    }
    if has_scheme_prefix(trimmed) {
        return false;
    }
    let host = strip_port(host);
    !host.contains(':')
        && (host.char_indices()).any(|(at, c)| c == '.' && at > 0 && at + 1 < host.len())
}

/// `value` as a full URL. Without a scheme it gets `http` for a local host
/// and `https` for any other. The scheme and host come out in lower case and
/// an empty path becomes `/`, as `new URL(value).toString()` gives.
pub fn normalize_user_url(value: &str) -> String {
    let trimmed = value.trim();
    let with_scheme = match trimmed.split_once("://") {
        Some((scheme, _)) if is_scheme(scheme) => trimmed.to_owned(),
        Some(_) | None => {
            let rest = trimmed.strip_prefix("//").unwrap_or(trimmed);
            let scheme = if is_local_host(strip_port(authority(rest))) {
                "http"
            } else {
                "https"
            };
            format!("{scheme}://{rest}")
        }
    };
    let Some((scheme, rest)) = with_scheme.split_once("://") else {
        return with_scheme;
    };
    let host = authority(rest);
    let after = &rest[host.len()..];
    let slash = if after.starts_with('/') { "" } else { "/" };
    format!(
        "{}://{}{slash}{after}",
        scheme.to_ascii_lowercase(),
        host.to_ascii_lowercase()
    )
}

/// What an address field does with `value`: a URL is completed as
/// [`normalize_user_url`] does, anything else becomes a web search for it.
/// `None` for an empty field. Ported from `resolveAddressInput`.
pub fn resolve_address_input(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if looks_like_url(trimmed) {
        return Some(normalize_user_url(trimmed));
    }
    Some(format!(
        "https://www.google.com/search?q={}",
        encode_component(trimmed)
    ))
}

/// `value` as `encodeURIComponent` writes it.
fn encode_component(value: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

fn strip_prefix_ignoring_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

/// `text` up to its path, query or fragment.
fn authority(text: &str) -> &str {
    text.split(['/', '?', '#']).next().unwrap_or(text)
}

/// `host` without a trailing `:port`.
fn strip_port(host: &str) -> &str {
    match host.rsplit_once(':') {
        Some((name, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => name,
        Some(_) | None => host,
    }
}

fn is_scheme(text: &str) -> bool {
    let mut characters = text.chars();
    characters.next().is_some_and(|c| c.is_ascii_alphabetic())
        && characters.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

/// Whether `text` starts with `scheme:`.
fn has_scheme_prefix(text: &str) -> bool {
    text.split_once(':')
        .is_some_and(|(scheme, _)| is_scheme(scheme))
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || matches!(host, "::1" | "[::1]")
}

fn is_local_host_with_port(host: &str) -> bool {
    is_loopback(host) || is_loopback(strip_port(host))
}

fn is_private_ipv4(host: &str) -> bool {
    let octets: Vec<u8> = host
        .split('.')
        .map_while(|part| part.parse().ok())
        .collect();
    if octets.len() != 4 || host.split('.').count() != 4 {
        return false;
    }
    match (octets[0], octets[1]) {
        (10 | 127, _) | (192, 168) | (169, 254) => true,
        (172, second) => (16..=31).contains(&second),
        _ => false,
    }
}

fn is_local_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let top_level = host.rsplit_once('.').map(|(_, label)| label);
    is_loopback(&host) || is_private_ipv4(&host) || top_level == Some("local")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_urls_and_bare_hosts_look_like_urls() {
        for url in [
            "https://example.com",
            "HTTP://Example.com/a?b#c",
            "example.com",
            "example.com/path?x=1",
            "sub.example.co.uk",
            "192.168.1.4:3000/app",
            "localhost",
            "localhost:4321/garden",
            "[::1]:8080",
            "  example.com  ",
        ] {
            assert!(looks_like_url(url), "{url}");
        }

        {
            for text in [
                "",
                "hello",
                "hello world.com",
                "mailto:me@example.com",
                "file:///tmp/a.png",
                "javascript:alert(1)",
                "https://",
                ".com",
                "example.",
                "a sentence. With a dot",
            ] {
                assert!(!looks_like_url(text), "{text}");
            }
        }
    }

    #[test]
    fn a_bare_host_gets_https_and_a_local_one_http() {
        assert_eq!(normalize_user_url("example.com"), "https://example.com/");
        assert_eq!(
            normalize_user_url("Example.com/Path?q=1"),
            "https://example.com/Path?q=1"
        );
        assert_eq!(
            normalize_user_url("localhost:4321/garden"),
            "http://localhost:4321/garden"
        );
        assert_eq!(
            normalize_user_url("192.168.1.4:3000"),
            "http://192.168.1.4:3000/"
        );
        assert_eq!(normalize_user_url("printer.local"), "http://printer.local/");
        assert_eq!(
            normalize_user_url("//example.com/a"),
            "https://example.com/a"
        );

        {
            assert_eq!(
                normalize_user_url(" HTTP://Example.com?x=1 "),
                "http://example.com/?x=1"
            );
            assert_eq!(
                normalize_user_url("https://example.com/a#b"),
                "https://example.com/a#b"
            );
        }
    }
}
