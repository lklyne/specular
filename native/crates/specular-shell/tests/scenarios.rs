//! Every headless scenario in `fixtures/scenarios/`, run as one test so
//! `cargo test --workspace` covers them. `SCENARIO=name` runs the ones whose
//! file name holds `name`. What a run saves and draws lands in
//! `target/tmp/scenarios/<name>/`.

#![expect(clippy::unwrap_used, reason = "a test fails loudly")]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use specular_compositor::{CompositorError, GpuContext};

fn scenarios() -> Vec<PathBuf> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/scenarios");
    let wanted = std::env::var("SCENARIO").ok();
    let mut found: Vec<PathBuf> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "txt"))
        .filter(|path| {
            let name = path.file_stem().unwrap().to_string_lossy();
            wanted.as_deref().is_none_or(|wanted| name.contains(wanted))
        })
        .collect();
    found.sort();
    found
}

#[test]
fn every_headless_scenario_passes() {
    match pollster::block_on(GpuContext::headless()) {
        Ok(_) => {}
        Err(CompositorError::NoAdapter(reason)) => {
            eprintln!("skipping the scenarios: no GPU adapter ({reason})");
            return;
        }
        Err(other) => panic!("GPU setup failed: {other}"),
    }
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("scenarios");
    let queue = Mutex::new(scenarios());
    let failed = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(2, |count| count.get().min(4));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let Some(script) = queue.lock().unwrap().pop() else {
                        return;
                    };
                    let name = script.file_stem().unwrap().to_string_lossy().into_owned();
                    let folder = out.join(&name);
                    let _ = std::fs::remove_dir_all(&folder);
                    std::fs::create_dir_all(&folder).unwrap();
                    match specular_shell::run_scenario(&script, &folder) {
                        Ok(report) if report.skipped.is_empty() => eprintln!("pass {name}"),
                        Ok(report) => eprintln!(
                            "pass {name} (skipped {} window steps)",
                            report.skipped.len()
                        ),
                        Err(error) => {
                            eprintln!("FAIL {name}: {error:#}");
                            failed.lock().unwrap().push(name);
                        }
                    }
                }
            });
        }
    });
    let failed = failed.into_inner().unwrap();
    assert!(failed.is_empty(), "scenarios failed: {}", failed.join(" "));
}
