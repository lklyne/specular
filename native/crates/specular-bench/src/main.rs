//! `specular-bench`: prints the gesture plan as JSON.
//!
//! The live runner (drive the app's camera through [`PROFILES`] while timing
//! presented frames, then emit one [`specular_bench::FrameSummary`] per
//! profile) is built on top of this.

use serde::Serialize;
use specular_bench::{PROFILES, ProfileId, STEP_INTERVAL, build_steps};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlannedProfile {
    id: ProfileId,
    label: &'static str,
    duration_ms: u128,
    steps: usize,
}

fn main() -> anyhow::Result<()> {
    let plan: Vec<PlannedProfile> = PROFILES
        .iter()
        .map(|profile| PlannedProfile {
            id: profile.id,
            label: profile.label,
            duration_ms: profile.duration.as_millis(),
            steps: build_steps(profile, STEP_INTERVAL).len(),
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}
