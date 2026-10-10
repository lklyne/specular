//! Minimal argument parsing: positionals plus `--flag value` pairs.

use std::{collections::HashMap, str::FromStr, time::Duration};

use anyhow::{Context as _, bail};
use specular_bench::{GestureProfile, ProfileId, select_profiles};

/// Parsed command line after the subcommand name.
#[derive(Debug, Default)]
pub(crate) struct Args {
    positionals: Vec<String>,
    flags: HashMap<String, String>,
}

impl Args {
    /// Splits `raw` into positionals and `--flag value` pairs.
    pub(crate) fn parse(raw: impl IntoIterator<Item = String>) -> anyhow::Result<Self> {
        let mut args = Self::default();
        let mut raw = raw.into_iter();
        while let Some(arg) = raw.next() {
            if let Some(flag) = arg.strip_prefix("--") {
                let value = raw
                    .next()
                    .with_context(|| format!("--{flag} needs a value"))?;
                args.flags.insert(flag.to_owned(), value);
            } else {
                args.positionals.push(arg);
            }
        }
        Ok(args)
    }

    /// Positional `index`, if given.
    pub(crate) fn positional(&self, index: usize) -> Option<&str> {
        self.positionals.get(index).map(String::as_str)
    }

    /// Every positional.
    pub(crate) fn positionals(&self) -> &[String] {
        &self.positionals
    }

    /// Raw value of `--name`.
    pub(crate) fn flag(&self, name: &str) -> Option<&str> {
        self.flags.get(name).map(String::as_str)
    }

    /// `--name` parsed as `T`.
    pub(crate) fn parsed<T>(&self, name: &str) -> anyhow::Result<Option<T>>
    where
        T: FromStr,
        T::Err: std::error::Error + Send + Sync + 'static,
    {
        self.flag(name)
            .map(|raw| raw.parse::<T>().with_context(|| format!("--{name} {raw}")))
            .transpose()
    }

    /// `--name` in milliseconds, as a duration.
    pub(crate) fn millis(&self, name: &str) -> anyhow::Result<Option<Duration>> {
        let Some(ms) = self.parsed::<f64>(name)? else {
            return Ok(None);
        };
        if !ms.is_finite() || ms < 0.0 {
            bail!("--{name} must be a non-negative number of milliseconds");
        }
        Ok(Some(Duration::from_secs_f64(ms / 1_000.0)))
    }

    /// Profiles selected by `--profiles a,b` and `--duration-ms`, mirroring
    /// the `profiles` and `durationMs` fields of `/perf/pan-zoom/run`.
    pub(crate) fn profiles(&self) -> anyhow::Result<Vec<GestureProfile>> {
        let ids = self
            .flag("profiles")
            .map(|list| {
                list.split(',')
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                    .map(str::parse::<ProfileId>)
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        Ok(select_profiles(&ids, self.millis("duration-ms")?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(raw: &[&str]) -> Args {
        Args::parse(raw.iter().map(|s| (*s).to_owned())).unwrap()
    }

    #[test]
    fn positionals_and_flags_are_separated() {
        let parsed = args(&["a.json", "--frame-ms", "8.33", "b.json"]);
        assert_eq!(
            (parsed.positional(1), parsed.flag("frame-ms")),
            (Some("b.json"), Some("8.33"))
        );
    }

    #[test]
    fn negative_millis_are_rejected() {
        assert!(args(&["--gap-ms", "-1"]).millis("gap-ms").is_err());
    }
}
