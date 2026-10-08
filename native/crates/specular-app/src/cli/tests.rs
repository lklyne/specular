use specular_bench::PROFILES;

use super::*;

fn parse_strs(args: &[&str]) -> anyhow::Result<Command> {
    parse(args.iter().map(OsString::from))
}

fn run_args(args: &[&str]) -> RunArgs {
    match parse_strs(args).unwrap() {
        Command::Run(run) => run,
        Command::Help => panic!("expected a run"),
    }
}

#[test]
fn space_choice_follows_the_flags_and_a_bare_path() {
    let run = run_args(&[]);
    assert_eq!((&run.canvas, run.pages, &run.bench), (&None, None, &None));
    let named = run_args(&["--space", "some/folder"]).space_choice();
    assert_eq!(named, SpaceChoice::Path(PathBuf::from("some/folder")));
    for (args, expected) in [
        (&[][..], SpaceChoice::Scratch),
        (&["--space", "user"], SpaceChoice::User),
        (&["--source", "synthetic"], SpaceChoice::Scratch),
        (&["some/folder"], named),
    ] {
        assert_eq!(run_args(args).space_choice(), expected, "{args:?}");
    }
}

#[test]
fn flags_set_their_fields() {
    let defaults = run_args(&[]);
    assert_eq!((defaults.chrome, defaults.annotations), (true, 0));
    assert_eq!(defaults.paint_policy, PaintPolicy::ElectronLod);
    assert_eq!(run_args(&["--pages", "20"]).pages, Some(20));
    assert_eq!(
        run_args(&["--source", "synthetic"]).source,
        SourceKind::Synthetic
    );
    assert_eq!(
        run_args(&["--window", "1600x1000"]).window,
        Some((1600, 1000))
    );
    assert!(!run_args(&["--chrome", "off"]).chrome);
    assert_eq!(run_args(&["--annotations", "40"]).annotations, 40);
    assert_eq!(
        run_args(&["--paint-policy", "full-rate"]).paint_policy,
        PaintPolicy::FullRate
    );
    assert_eq!(
        run_args(&["demo.canvas"]).canvas,
        Some(PathBuf::from("demo.canvas"))
    );
    let run = run_args(&["--bench", "all", "--warmup-ms", "8000"]);
    assert_eq!(run.warmup, Duration::from_secs(8));
}

#[test]
fn bad_flags_are_refused() {
    for args in [
        &["--space", "user", "a.canvas"][..],
        &["--space"],
        &["--pages", "0"],
        &["--pages"],
        &["demo.canvas", "--pages", "3"],
        &["--source", "webkit"],
        &["--window", "0x600"],
        &["--chrome", "maybe"],
        &["--annotations", "lots"],
        &["--chrome", "off", "--annotations", "5"],
        &["--annotations", "5", "--chrome", "off"],
        &["--paint-policy", "fast"],
        &["--bench", "spin"],
        &["--snapshot-camera", "near"],
        &["--snapshot-scale", "0"],
        &["--bench-target", "headless"],
        &["--bench", "idle", "--bench-target", "tv"],
    ] {
        assert!(parse_strs(args).is_err(), "{args:?}");
    }
}

#[test]
fn help_flag_wins_over_other_arguments() {
    assert_eq!(parse_strs(&["--pages", "3", "-h"]).unwrap(), Command::Help);
}

#[test]
fn bench_selects_profiles_in_electron_order() {
    let ids = |args: &[&str]| -> Vec<_> {
        run_args(args)
            .bench
            .unwrap()
            .iter()
            .map(|profile| profile.id)
            .collect()
    };
    assert_eq!(ids(&["--bench", "all"]), PROFILES.map(|profile| profile.id));
    assert!(!ids(&["--bench", "all"]).contains(&ProfileId::Idle));
    assert_eq!(ids(&["--bench", "fast-pan-zoom"]), [ProfileId::FastPanZoom]);
    assert_eq!(
        ids(&["--bench", "slow-zoom,slow-pan"]),
        [ProfileId::SlowPan, ProfileId::SlowZoom]
    );
    let named = run_args(&["--bench", "slow-pan,idle", "--bench-duration-ms", "500"]);
    let durations: Vec<_> = named
        .bench
        .unwrap()
        .iter()
        .map(|p| (p.id, p.duration))
        .collect();
    let half = Duration::from_millis(500);
    assert_eq!(
        durations,
        [(ProfileId::SlowPan, half), (ProfileId::Idle, half)]
    );
    assert!(run_args(&["--bench", "idle", "--bench-target", "headless"]).bench_headless);
}

#[test]
fn snapshot_flags_ask_for_a_headless_run() {
    let run = run_args(&[
        "--snapshot",
        "out.png",
        "--snapshot-size",
        "800x600",
        "--snapshot-camera",
        "10,20,3",
        "--snapshot-scale",
        "2",
        "sink.canvas",
    ]);
    let camera = specular_core::Camera::new(glam::Vec2::new(10.0, 20.0), 3.0);
    assert!(run.headless.is_requested());
    assert_eq!(
        run.headless,
        HeadlessArgs {
            snapshot: Some(PathBuf::from("out.png")),
            size: (800, 600),
            scale: 2.0,
            camera: headless::CameraArg::At(camera),
            script: None,
            source: SourceKind::Synthetic,
        }
    );
}

#[test]
fn headless_runs_follow_the_snapshot_and_script_flags() {
    let unnamed = run_args(&["--snapshot", "out.png"]);
    assert_eq!(unnamed.headless.source, SourceKind::Synthetic);
    let named = run_args(&["--source", "cef", "--snapshot", "out.png"]);
    assert_eq!(named.headless.source, SourceKind::Cef);
    let run = run_args(&["--script", "steps.txt"]);
    assert!(run.headless.is_requested());
    assert_eq!(run.headless.camera, headless::CameraArg::Fit);
    assert!(!run_args(&["sink.canvas"]).headless.is_requested());
}
