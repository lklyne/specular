//! The verbs over the app's command registry and keyed state: `commands`
//! lists every command the app can run by name, `run <name> [json-args]`
//! runs one, and `state [key]` reads what a mod reads.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::Client;
use crate::{output, refuse, usage};

/// `commands`: each command's name, arguments, what it does, and whether
/// it can run now.
pub(crate) fn commands(client: &Client, _args: &Args) -> anyhow::Result<i32> {
    output::json(&client.get("/commands")?);
    Ok(0)
}

/// `run <name> [json-args]`: one command, by name, as one undo step.
pub(crate) fn run(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let Some(name) = args.nth(0) else {
        return usage("specular run <name> [json-args]");
    };
    let text = args.words(1);
    let command_args = if text.is_empty() {
        Value::Null
    } else {
        match serde_json::from_str::<Value>(&text) {
            Ok(value) => value,
            Err(error) => return refuse(&format!("the arguments are not JSON: {error}")),
        }
    };
    let body = json!({ "name": name, "args": command_args });
    output::json(&client.post("/run", &body)?);
    Ok(0)
}

/// `state [key]`: every key of the app's keyed state, or the one named.
pub(crate) fn state(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let path = match args.nth(0) {
        Some(key) => format!("/state?key={}", encode(key)),
        None => "/state".to_owned(),
    };
    output::json(&client.get(&path)?);
    Ok(0)
}

/// `key` as a query value: a state key is lower case words, `/` and `_`,
/// and anything else is escaped.
fn encode(key: &str) -> String {
    key.bytes()
        .map(|byte| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'/' | b'_' | b'-' | b'.' => {
                char::from(byte).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}
