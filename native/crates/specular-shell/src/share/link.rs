//! A share link, `<server>/c/<doc id>#t=<link token>`, and what it opens.

/// A share link, taken apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShareLink {
    /// The server, `https://host` with no trailing slash.
    pub(crate) server: String,
    /// The doc id.
    pub(crate) doc: String,
    /// The link token, which the server trades for a connection token.
    pub(crate) token: String,
}

impl ShareLink {
    /// `link` taken apart, or why it is not a share link.
    pub(crate) fn parse(link: &str) -> Result<Self, String> {
        let bad = || format!("not a share link: {link}");
        let (address, token) = link.trim().split_once("#t=").ok_or_else(bad)?;
        let (server, doc) = address.split_once("/c/").ok_or_else(bad)?;
        let doc = doc.trim_end_matches('/');
        let plain = |text: &str| {
            !text.is_empty()
                && text
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        };
        let web = server.starts_with("https://") || server.starts_with("http://");
        if !web || !plain(doc) || !plain(token) {
            return Err(bad());
        }
        Ok(Self {
            server: server.trim_end_matches('/').to_owned(),
            doc: doc.to_owned(),
            token: token.to_owned(),
        })
    }

    /// The room's websocket address for the connection token `token`.
    pub(crate) fn live_url(&self, token: &str) -> String {
        let socket = (self
            .server
            .strip_prefix("https://")
            .map(|rest| format!("wss://{rest}")))
        .or_else(|| (self.server.strip_prefix("http://")).map(|rest| format!("ws://{rest}")))
        .unwrap_or_else(|| self.server.clone());
        format!("{socket}/docs/{}/live?token={token}", self.doc)
    }
}

#[cfg(test)]
mod tests {
    use super::ShareLink;

    #[test]
    fn a_link_names_its_server_doc_and_token() {
        let link = ShareLink::parse("https://specular.example/c/AbC_1#t=tok-9").expect("a link");
        assert_eq!(link.server, "https://specular.example");
        assert_eq!(link.doc, "AbC_1");
        assert_eq!(link.token, "tok-9");
        assert_eq!(
            link.live_url("conn"),
            "wss://specular.example/docs/AbC_1/live?token=conn"
        );
        let local = ShareLink::parse("http://localhost:8787/c/d#t=t").expect("a link");
        assert_eq!(
            local.live_url("c"),
            "ws://localhost:8787/docs/d/live?token=c"
        );
        for bad in [
            "https://x/c/d",
            "https://x/d#t=t",
            "ftp://x/c/d#t=t",
            "https://x/c/d#t=a b",
        ] {
            assert!(ShareLink::parse(bad).is_err(), "{bad}");
        }
    }
}
