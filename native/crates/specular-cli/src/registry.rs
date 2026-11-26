//! The verbs over the app's command registry: `commands` lists every
//! command the app can run by name, and `run <name> [json-args]` runs one.

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
