//! The executable inside each `<App> Helper*.app`.
//!
//! Chromium starts every child (renderer, GPU, network and other utilities)
//! by launching a helper bundle with `--type=<kind>`. This is that program:
//! it loads the framework, runs the child to completion in
//! `CefExecuteProcess` and exits with its code. It links `cef` and nothing
//! else, so the five helper bundles cost a few hundred kilobytes each rather
//! than a copy of the app.
//!
//! A child gets no `App`. The app's only `App` hooks are the browser
//! process's (its command-line switches and the message pump), and Chromium
//! forwards the switches a child needs on its command line. There is no
//! render process handler and no custom scheme. Add one here, and to
//! `specular_cef::run_subprocess_if_needed`, the day the app grows one.
//!
//! The macOS sandbox is not started, as the app runs CEF with `no_sandbox`.

#[cfg(feature = "cef")]
mod child;

fn main() {
    #[cfg(feature = "cef")]
    let code = child::run();
    #[cfg(not(feature = "cef"))]
    let code = {
        eprintln!("specular-cef-helper was built without the `cef` feature and runs nothing");
        1
    };
    std::process::exit(code);
}
