//! One real run through `ClaudeCli` and `AgentRuns`, printing every notice.
//!
//! `cargo run -p specular-app --example agent_once -- <dir> <prompt..>
//! [--resume <session id>] [--image <path relative to dir>]`

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use specular_agent::{Image, MediaType, Notice, RunRequest, ThreadId};
use specular_app::agent::{AgentRuns, ClaudeCli};

fn media_type(path: &str) -> Option<MediaType> {
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some(MediaType::Png),
        "jpg" | "jpeg" => Some(MediaType::Jpeg),
        "gif" => Some(MediaType::Gif),
        "webp" => Some(MediaType::Webp),
        _ => None,
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(dir) = args.next().map(PathBuf::from) else {
        eprintln!("usage: agent_once <dir> <prompt..> [--resume <id>] [--image <path>]");
        return ExitCode::from(2);
    };
    let (mut prompt, mut resume, mut images) = (Vec::new(), None, Vec::new());
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--resume" => resume = args.next(),
            "--image" => {
                let Some(path) = args.next() else { break };
                let Some(media_type) = media_type(&path) else {
                    eprintln!("not an image type: {path}");
                    return ExitCode::from(2);
                };
                images.push(Image { path, media_type });
            }
            _ => prompt.push(arg),
        }
    }
    let thread = ThreadId("once".into());
    let request = RunRequest {
        thread,
        prompt: prompt.join(" "),
        resume,
        images,
    };
    let mut runs = AgentRuns::new(Box::new(ClaudeCli::new()));
    runs.start(&request, Some(&dir));
    let mut finished = false;
    while !runs.is_idle() {
        for (_, notice) in runs.poll() {
            println!("{notice:?}");
            finished |= matches!(notice, Notice::Finished { .. });
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if finished {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
