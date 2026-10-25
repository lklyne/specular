//! A browse command as Specular reads it before agent-browser runs it:
//! the verb, the `@eN` ref, what the cursor should say, and the element a
//! selector or locator names.

use serde_json::{Value, json};

/// agent-browser's flags that take a value, skipped when looking for the
/// verb and its positionals.
const VALUE_FLAGS: &[&str] = &[
    "--cdp",
    "--session",
    "--viewport",
    "--timeout",
    "--selector",
    "--format",
    "--depth",
    "--wait",
    "--attr",
    "--baseline",
    "--screenshot-format",
    "--screenshot-quality",
    "--screenshot-dir",
    "--max-output",
    "--download-path",
    "--executable-path",
    "--extension",
    "--headers",
    "--body",
    "--filter",
    "--profile",
    "--session-name",
    "--device",
    "--color-scheme",
    "--idle-timeout",
    "--name",
    "-s",
    "-d",
    "-p",
];

/// The verbs that change a page, and so get a pre-scroll, a staleness
/// check and the address after.
const MUTATIONS: &[&str] = &["click", "fill", "type", "select"];

/// The locators `find` takes before its value and action.
const FIND_LOCATORS: &[&str] = &[
    "role",
    "text",
    "label",
    "placeholder",
    "alt",
    "title",
    "testid",
    "first",
    "last",
];

/// `find` actions that are a click.
const CLICK_LIKE: &[&str] = &["check", "uncheck", "dblclick", "tap"];

/// The verbs that would take the browser or the page's address away from
/// Specular, and what to use instead.
pub(crate) fn blocked(verb: &str) -> Option<&'static str> {
    const PAGES_IN_PLACE: &str =
        "pages are driven in place: `specular add page <url>` then `specular snapshot -f PAGE_ID`.";
    const LIFECYCLE: &str =
        "Specular owns browser lifecycle; to remove a page use `specular delete <id>`.";
    const BUNDLED: &str = "the agent-browser driver is bundled with Specular.";
    Some(match verb {
        "launch" | "connect" => PAGES_IN_PLACE,
        "close" | "quit" => LIFECYCLE,
        "install" | "upgrade" => BUNDLED,
        "open" => {
            "a CDP-driven navigation doesn't update the page entity's URL in the workspace; use \
             `specular update PAGE_ID --url <url>` instead."
        }
        _ => return None,
    })
}

/// The cursor's label for what an effective verb does.
fn label(verb: &str) -> Option<&'static str> {
    Some(match verb {
        "snapshot" | "eval" | "focus" => "inspect_page",
        "click" => "click_target",
        "fill" | "type" | "keyboard" => "type_text",
        "select" => "select_option",
        "wait" => "wait_page",
        "scroll" => "scroll_page",
        "get" | "clipboard" => "read_content",
        "query-elements" | "find" => "find_target",
        "screenshot" => "take_screenshot",
        _ => return None,
    })
}

/// Whether `word` is a snapshot ref such as `@e12`.
pub(crate) fn is_ref(word: &str) -> bool {
    word.strip_prefix("@e")
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// A command split into what Specular needs from it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Parsed {
    /// The words as agent-browser gets them.
    pub(crate) argv: Vec<String>,
    /// The first word that is not a flag.
    pub(crate) verb: Option<String>,
    /// The first `@eN` after the verb.
    pub(crate) target_ref: Option<String>,
    /// The words after the verb that are not flags or their values.
    pub(crate) positionals: Vec<String>,
}

impl Parsed {
    /// Reads `argv`.
    pub(crate) fn new(argv: Vec<String>) -> Self {
        let mut parsed = Self::default();
        let mut at = 0;
        while let Some(word) = argv.get(at) {
            if VALUE_FLAGS.contains(&word.as_str()) {
                at += 2;
                continue;
            }
            at += 1;
            if word.starts_with('-') {
                continue;
            }
            if parsed.verb.is_none() {
                parsed.verb = Some(word.clone());
                continue;
            }
            if parsed.target_ref.is_none() && is_ref(word) {
                parsed.target_ref = Some(word.clone());
            }
            parsed.positionals.push(word.clone());
        }
        parsed.argv = argv;
        parsed
    }

    fn nth(&self, index: usize) -> Option<&str> {
        self.positionals.get(index).map(String::as_str)
    }

    /// The verb as the cursor should read it: `find text "Go" click` is a
    /// click.
    pub(crate) fn effective(&self) -> Option<&str> {
        let verb = self.verb.as_deref()?;
        if verb != "find" {
            return Some(verb);
        }
        let action = match self.nth(0) {
            Some("nth") => self.nth(3),
            Some(locator) if FIND_LOCATORS.contains(&locator) => self.nth(2),
            _ => None,
        };
        Some(match action {
            Some(action) if MUTATIONS.contains(&action) => action,
            Some(action) if CLICK_LIKE.contains(&action) => "click",
            _ => "find",
        })
    }

    /// The mutation this command makes, if any.
    pub(crate) fn mutation(&self) -> Option<&str> {
        self.effective().filter(|verb| MUTATIONS.contains(verb))
    }

    /// The cursor's label for it.
    pub(crate) fn label(&self) -> Option<&'static str> {
        self.effective().and_then(label)
    }

    /// The element a selector, a `text=` locator or a `find role|testid`
    /// locator names, for the cursor to move to before agent-browser finds
    /// it. `None` for a ref, and for a verb with no target.
    pub(crate) fn target_query(&self) -> Option<Value> {
        let verb = self.verb.as_deref()?;
        if verb == "find" {
            let (kind, value) = (self.nth(0)?, self.nth(1)?);
            let name = (self.argv.iter().position(|word| word == "--name"))
                .and_then(|at| self.argv.get(at + 1));
            let selector = match kind {
                "role" => format!("[role=\"{value}\"]"),
                "testid" => format!("[data-testid=\"{value}\"]"),
                _ => return None,
            };
            return Some(json!({ "selector": selector, "text": null, "name": name }));
        }
        if !MUTATIONS.contains(&verb) || self.target_ref.is_some() {
            return None;
        }
        let target = self.nth(0).filter(|target| !is_ref(target))?;
        Some(match target.strip_prefix("text=") {
            Some(text) => json!({ "selector": null, "text": text, "name": null }),
            None => json!({ "selector": target, "text": null, "name": null }),
        })
    }

    /// What to scroll into view before a mutation: its ref or selector.
    /// agent-browser's `scrollintoview` has no text locator.
    pub(crate) fn scroll_target(&self) -> Option<&str> {
        if !self
            .verb
            .as_deref()
            .is_some_and(|verb| MUTATIONS.contains(&verb))
        {
            return None;
        }
        if let Some(target) = &self.target_ref {
            return Some(target);
        }
        self.nth(0).filter(|target| !target.starts_with("text="))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(line: &str) -> Parsed {
        Parsed::new(line.split(' ').map(str::to_owned).collect())
    }

    #[test]
    fn a_find_locator_reads_as_its_action_and_names_its_element() {
        let find = parsed("find role button click --name Submit");
        assert_eq!(find.mutation(), Some("click"));
        assert_eq!(
            find.target_query(),
            Some(json!({ "selector": "[role=\"button\"]", "text": null, "name": "Submit" }))
        );
        assert_eq!(parsed("find text Next check").effective(), Some("click"));
        assert_eq!(parsed("find text Next hover").label(), Some("find_target"));

        let click = parsed("click text=Save");
        assert_eq!(click.scroll_target(), None);
        assert_eq!(
            click.target_query(),
            Some(json!({ "selector": null, "text": "Save", "name": null }))
        );
        let by_ref = parsed("fill @e7 hello -s ignored");
        assert_eq!(
            (by_ref.target_query(), by_ref.scroll_target()),
            (None, Some("@e7"))
        );
    }
}
