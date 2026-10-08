//! Repo bindings in a test: which folder a site's origin writes to.

use specular_interact::{Event, Repos};

use crate::TestApp;

impl TestApp {
    /// Replaces the connected repos, as loading `repos.json` does. No effect
    /// comes back, so a test needs no `take_effects` after it.
    pub fn with_repos(&mut self, repos: Repos) -> &mut Self {
        self.send(Event::ReposLoaded(Box::new(repos)))
    }

    /// Binds `origin` to the repo at `path` (connecting it), with auto-fix
    /// `auto`, on top of the repos already there.
    pub fn bind(&mut self, origin: &str, path: &str, auto: bool) -> &mut Self {
        let mut repos = self.app().repos().clone();
        repos.bind(origin, path);
        repos.set_auto_fix(origin, auto);
        self.with_repos(repos)
    }
}
