//! One loaded mod: its runtime, its view and what it costs.
//!
//! Each mod gets a runtime of its own. That keeps one mod's globals out of
//! another's and gives [`ModStats`] per mod, at the price of a `QuickJS` heap
//! each, which is small beside the window.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Context as _;
use gpui::{AnyView, App, Entity, Window};
use gpui_shell::policy::{self, Policy};
use gpui_shell::{RuntimeMetrics, ScriptView, ShellRoot, ShellRuntime};

use crate::host::{Hosting, specular_module};
use crate::manifest::read_manifest;
use specular_interact::mods::ModManifest;

/// A loaded mod.
pub struct Mod {
    id: String,
    dir: PathBuf,
    manifest: ModManifest,
    runtime: Rc<ShellRuntime>,
    root: Entity<ShellRoot>,
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

/// The authority the mod `id` runs under: everything the person has, its
/// own data folder, and the `specular` module.
fn policy_for(id: &str, dir: &Path, data_dir: &Path, hosting: &Hosting) -> anyhow::Result<Policy> {
    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("creating {}", data_dir.display()))?;
    Policy::new()
        .with_application(id)
        .with_capabilities(crate::grant::trusted(dir, data_dir))
        .with_data_dir(data_dir.to_path_buf())
        .with_storage_path(data_dir.join("store.json"))
        .with_host_module(specular_module(id, hosting))
        .map_err(|error| anyhow::anyhow!("{}", error.message()))
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
    /// state. [`ModSet`](crate::ModSet) loads it again when its source
    /// changes.
    pub fn load(
        id: &str,
        dir: &Path,
        data_dir: &Path,
        hosting: &Hosting,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<Self> {
        let manifest = read_manifest(dir)?;
        let runtime = ShellRuntime::new_isolated()?;
        let policy = policy_for(id, dir, data_dir, hosting)?;
        hosting.reads.borrow_mut().begin(id);
        let root = under(policy, || runtime.try_load(dir, window, cx))
            .with_context(|| format!("loading mod `{id}`"))?;
        Ok(Self {
            id: id.to_owned(),
            dir: dir.to_path_buf(),
            manifest,
            runtime,
            root,
        })
    }

    /// Loads the mod in `dir` and renders it once in `window`, which it
    /// takes over, returning the element tree it described. A load, a
    /// render or a host component that fails is the error.
    pub fn check(
        id: &str,
        dir: &Path,
        data_dir: &Path,
        hosting: &Hosting,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<String> {
        read_manifest(dir)?;
        let runtime = ShellRuntime::new_isolated()?;
        let policy = policy_for(id, dir, data_dir, hosting)?;
        under(policy, || runtime.check(dir, window, cx))
            .with_context(|| format!("checking mod `{id}`"))
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

    /// The mod's view, to place in a region. Placed without the runtime's
    /// own window root, which paints a background over its whole box and
    /// would hold its view's description past a camera move.
    pub fn view(&self, cx: &App) -> AnyView {
        self.root.read(cx).content().clone()
    }

    /// The element tree its last script render described, with ids: what
    /// `specular mod check` prints. `Err` is why the render failed.
    pub fn tree(&self, cx: &App) -> Result<String, String> {
        let view = (self.view(cx).downcast::<ScriptView>())
            .map_err(|_| "the mod failed to load".to_owned())?;
        let view = view.read(cx);
        if let Some(error) = view.build_error() {
            return Err(error.to_owned());
        }
        view.snapshot()
            .map(gpui_shell::RenderSnapshot::debug_tree)
            .ok_or_else(|| "the mod has not rendered yet".to_owned())
    }

    /// Renders the mod again if a key it read has moved. Returns whether it
    /// will.
    pub fn refresh_if_stale(&self, hosting: &Hosting, cx: &mut App) -> bool {
        let reads = &hosting.reads;
        if !reads.borrow().stale(&self.id, &hosting.revisions.borrow()) {
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
