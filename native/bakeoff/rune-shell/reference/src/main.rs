//! `specular --snapshot` and `--script` on a machine where the `specular`
//! binary does not build: the shell library's own command line, headless.

fn main() -> anyhow::Result<()> {
    if specular_shell::launch(std::env::args_os().skip(1))?.is_some() {
        anyhow::bail!("only --snapshot and --script runs: there is no window here");
    }
    Ok(())
}
