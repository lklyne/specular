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
fn no_arguments_runs_default_demo() {
    let run = run_args(&[]);
    assert_eq!((run.canvas, run.pages, run.bench), (None, None, None));
}

#[test]
fn pages_flag_sets_demo_page_count() {
    assert_eq!(run_args(&["--pages", "20"]).pages, Some(20));
}

#[test]
fn zero_pages_is_rejected() {
    assert!(parse_strs(&["--pages", "0"]).is_err());
}

#[test]
fn source_flag_selects_synthetic() {
    assert_eq!(
        run_args(&["--source", "synthetic"]).source,
        SourceKind::Synthetic
    );
}

#[test]
fn unknown_source_is_rejected() {
    assert!(parse_strs(&["--source", "webkit"]).is_err());
}

#[test]
fn bench_all_selects_every_profile_in_order() {
    let ids: Vec<_> = run_args(&["--bench", "all"])
        .bench
        .unwrap()
        .iter()
        .map(|profile| profile.id)
        .collect();
    assert_eq!(ids, PROFILES.map(|profile| profile.id));
}

#[test]
fn bench_accepts_electron_profile_id() {
    let bench = run_args(&["--bench", "fast-pan-zoom"]).bench.unwrap();
    assert_eq!(bench[0].id, ProfileId::FastPanZoom);
}

#[test]
fn bench_list_runs_in_electron_order() {
    let ids: Vec<_> = run_args(&["--bench", "slow-zoom,slow-pan"])
        .bench
        .unwrap()
        .iter()
        .map(|profile| profile.id)
        .collect();
    assert_eq!(ids, [ProfileId::SlowPan, ProfileId::SlowZoom]);
}

#[test]
fn warmup_ms_sets_bench_warmup() {
    let run = run_args(&["--bench", "all", "--warmup-ms", "8000"]);
    assert_eq!(run.warmup, Duration::from_secs(8));
}

#[test]
fn window_flag_sets_logical_size() {
    assert_eq!(
        run_args(&["--window", "1600x1000"]).window,
        Some((1600, 1000))
    );
}

#[test]
fn window_flag_rejects_a_zero_side() {
    assert!(parse_strs(&["--window", "0x600"]).is_err());
}

#[test]
fn chrome_defaults_on_without_annotations() {
    let run = run_args(&[]);
    assert_eq!((run.chrome, run.annotations), (true, 0));
}

#[test]
fn chrome_flag_turns_the_layer_off() {
    assert!(!run_args(&["--chrome", "off"]).chrome);
}

#[test]
fn unknown_chrome_value_is_rejected() {
    assert!(parse_strs(&["--chrome", "maybe"]).is_err());
}

#[test]
fn annotations_flag_sets_the_seed_count() {
    assert_eq!(run_args(&["--annotations", "40"]).annotations, 40);
}

#[test]
fn annotations_with_chrome_off_is_rejected() {
    assert!(parse_strs(&["--chrome", "off", "--annotations", "5"]).is_err());
    assert!(parse_strs(&["--annotations", "5", "--chrome", "off"]).is_err());
}

#[test]
fn annotations_must_be_a_number() {
    assert!(parse_strs(&["--annotations", "lots"]).is_err());
}

#[test]
fn paint_policy_defaults_to_electron_lod() {
    assert_eq!(run_args(&[]).paint_policy, PaintPolicy::ElectronLod);
}

#[test]
fn paint_policy_flag_selects_full_rate() {
    assert_eq!(
        run_args(&["--paint-policy", "full-rate"]).paint_policy,
        PaintPolicy::FullRate
    );
}

#[test]
fn unknown_paint_policy_is_rejected() {
    assert!(parse_strs(&["--paint-policy", "fast"]).is_err());
}

#[test]
fn only_cef_frames_are_representative() {
    assert_eq!(
        [SourceKind::Cef, SourceKind::Synthetic].map(SourceKind::is_representative),
        [true, false]
    );
}

#[test]
fn unknown_bench_profile_is_rejected() {
    assert!(parse_strs(&["--bench", "spin"]).is_err());
}

#[test]
fn positional_argument_is_canvas_path() {
    assert_eq!(
        run_args(&["demo.canvas"]).canvas,
        Some(PathBuf::from("demo.canvas"))
    );
}

#[test]
fn pages_with_canvas_file_is_rejected() {
    assert!(parse_strs(&["demo.canvas", "--pages", "3"]).is_err());
}

#[test]
fn flag_missing_value_is_rejected() {
    assert!(parse_strs(&["--pages"]).is_err());
}

#[test]
fn help_flag_wins_over_other_arguments() {
    assert_eq!(parse_strs(&["--pages", "3", "-h"]).unwrap(), Command::Help);
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
        }
    );
}

#[test]
fn a_script_alone_is_a_headless_run_that_fits_the_document() {
    let run = run_args(&["--script", "steps.txt"]);
    assert!(run.headless.is_requested());
    assert_eq!(run.headless.camera, headless::CameraArg::Fit);
}

#[test]
fn a_plain_run_opens_a_window() {
    assert!(!run_args(&["sink.canvas"]).headless.is_requested());
}

#[test]
fn a_bad_snapshot_camera_is_rejected() {
    assert!(parse_strs(&["--snapshot-camera", "near"]).is_err());
    assert!(parse_strs(&["--snapshot-scale", "0"]).is_err());
}
