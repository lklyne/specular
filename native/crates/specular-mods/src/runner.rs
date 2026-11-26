//! One loaded mod: its runtime, its view and what it costs.
//!
//! Each mod gets a runtime of its own. That keeps one mod's globals out of
//! another's and gives [`ModStats`] per mod, at the price of a `QuickJS` heap
//! each, which is small beside the window.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Context as _;
use gpui::{App, Entity, Window};
use gpui_shell::policy::{self, Policy};
use gpui_shell::{RuntimeMetrics, ShellRoot, ShellRuntime, Watcher};

use crate::host::{ModHost, specular_module};
use crate::manifest::ModManifest;
use crate::reads::{Reads, Revisions};

/// A loaded mod.
pub struct Mod {
    id: String,
    dir: PathBuf,
    manifest: ModManifest,
    runtime: Rc<ShellRuntime>,
    root: Entity<ShellRoot>,
    _watcher: Option<Watcher>,
}

impl std::fmt::Debug for Mod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mod")
            .field("id", &self.id)
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

/// What a mod has cost since it loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModStats {
    /// Script renders.
    pub renders: u64,
    /// Time in script renders, all told.
    pub render_time: std::time::Duration,
    /// The slowest script render.
    pub slowest: std::time::Duration,
    /// Native calls from script, all told.
    pub native_time: std::time::Duration,
}

impl From<RuntimeMetrics> for ModStats {
    fn from(metrics: RuntimeMetrics) -> Self {
        Self {
            renders: metrics.script_renders(),
            render_time: metrics.script_render_time(),
            slowest: metrics.slowest_script_render(),
            native_time: metrics.native_time(),
        }
    }
}

/// Runs `body` with `policy` as the default, then puts back the empty
/// default. `gpui-shell` reads the default when it loads and mounts a
/// script, and the view keeps the policy it was mounted under.
fn under<R>(policy: Policy, body: impl FnOnce() -> R) -> R {
    policy::set_default(policy);
    let out = body();
    policy::set_default(Policy::new());
    out
}

impl Mod {
    /// Loads the mod in `dir` under the id `id`, with `data_dir` for its own
    /// state. With `watch`, a save to its files reloads it in place; a load
    /// that fails keeps the old build running and shows why.
    #[expect(
        clippy::too_many_arguments,
        reason = "the host's parts, passed once at load"
    )]
    pub fn load(
        id: &str,
        dir: &Path,
        data_dir: &Path,
        host: Rc<dyn ModHost>,
        revisions: Rc<RefCell<Revisions>>,
        reads: &Rc<RefCell<Reads>>,
        watch: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<Self> {
        let manifest = ModManifest::read(dir)?;
        std::fs::create_dir_all(data_dir)
            .with_context(|| format!("creating {}", data_dir.display()))?;
        let module = specular_module(id, host, revisions, reads.clone());
        let policy = Policy::new()
            .with_application(id)
            .with_capabilities(crate::grant::trusted(dir, data_dir))
            .with_data_dir(data_dir.to_path_buf())
            .with_storage_path(data_dir.join("store.json"))
            .with_host_module(module)
            .map_err(|error| anyhow::anyhow!("{}", error.message()))?;
        let runtime = ShellRuntime::new_isolated()?;
        reads.borrow_mut().begin(id);
        let root = under(policy, || runtime.try_load(dir, window, cx))
            .with_context(|| format!("loading mod `{id}`"))?;
        let watcher = if watch {
            match runtime.watch(&root, window, cx) {
                Ok(watcher) => Some(watcher),
                Err(error) => {
                    tracing::warn!("mod `{id}` will not reload on save: {error:#}");
                    None
                }
            }
        } else {
            None
        };
        Ok(Self {
            id: id.to_owned(),
            dir: dir.to_path_buf(),
            manifest,
            runtime,
            root,
            _watcher: watcher,
        })
    }

    /// The mod's id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Where it was loaded from.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Its manifest.
    pub fn manifest(&self) -> &ModManifest {
        &self.manifest
    }

    /// The element a region places: the mod's root view.
    pub fn root(&self) -> &Entity<ShellRoot> {
        &self.root
    }

    /// Renders the mod again if a key it read has moved. Returns whether it
    /// will.
    pub fn refresh_if_stale(
        &self,
        revisions: &Revisions,
        reads: &RefCell<Reads>,
        cx: &mut App,
    ) -> bool {
        if !reads.borrow().stale(&self.id, revisions) {
            return false;
        }
        reads.borrow_mut().begin(&self.id);
        if let Err(error) = self.runtime.refresh(&self.root, cx) {
            tracing::warn!("mod `{}` did not refresh: {error:#}", self.id);
        }
        true
    }

    /// What it has cost since it loaded.
    pub fn stats(&self) -> ModStats {
        self.runtime.read_metrics().into()
    }
}
