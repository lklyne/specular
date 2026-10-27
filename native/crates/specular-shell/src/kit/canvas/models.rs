//! The pure models GPUI draws from, and when they are read again.

use std::time::Instant;

use specular_interact::{
    Appearance, ChatModel, ControlsModel, Menu, OnboardingModel, SidebarModel, Theme, ToolbarModel,
    ViewStrip, chat, dock, menus, onboarding, sidebar, tool_options, toolbar, view_strip,
};
use specular_shell::Runtime;

use super::Canvas;
use crate::surface::CanvasSurface;

/// What GPUI draws from, as `update` last left it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Models {
    /// The tabs of the tab row: the canvas, then its pages and Documents.
    pub(crate) strip: ViewStrip,
    /// The tool buttons and the zoom readout.
    pub(crate) toolbar: ToolbarModel,
    /// The options of the tool in hand, shown beside its button.
    pub(crate) tool_options: Option<ControlsModel>,
    /// What the dock shows: the controls of the selection, while no tool
    /// with options is in hand.
    pub(crate) dock: Option<ControlsModel>,
    /// The left sidebar.
    pub(crate) sidebar: SidebarModel,
    /// The right panel: the canvas's agent threads and the composer.
    pub(crate) chat: ChatModel,
    /// The menu bar's menus that come from the app.
    pub(crate) menus: Vec<Menu>,
    /// The first-run view, while no space is open.
    pub(crate) onboarding: Option<OnboardingModel>,
    /// Light or dark: what the Kit's theme has to be.
    pub(crate) appearance: Appearance,
    /// The choice behind it, which `AppKit` is told of.
    pub(crate) theme: Theme,
}

impl Models {
    pub(super) fn of(runtime: &Runtime<CanvasSurface>) -> Self {
        let app = runtime.app();
        let tool_options = tool_options(app);
        Self {
            strip: view_strip(app),
            toolbar: toolbar(app),
            tool_options: tool_options.clone(),
            dock: if tool_options.is_some() {
                None
            } else {
                dock(app)
            },
            sidebar: sidebar(app),
            chat: chat(app),
            menus: menus(app),
            onboarding: onboarding(app),
            appearance: app.appearance(),
            theme: app.theme(),
        }
    }
}

impl Canvas {
    /// The app may have changed: brings the models in step and wakes GPUI
    /// if one changed, now or when the gate next opens.
    pub(crate) fn refresh_models(&mut self) {
        let now = Instant::now();
        self.rest.owed(now);
        self.pause_link(false);
        self.stale = true;
        self.gate.ask();
        self.sync_models(now);
    }

    /// Reads the models if the gate lets it and wakes GPUI when one changed
    /// or an effect asked something of the window. Called every turn.
    pub(super) fn sync_models(&mut self, now: Instant) {
        if self.gate.open(now) {
            self.read_models();
            if std::mem::take(&mut self.unseen) {
                self.wake_now();
            }
        }
        let menu = &self.runtime.app().session().panel.menu;
        if *menu != self.menu {
            self.menu.clone_from(menu);
            self.wake_now();
        }
        let asked = self.runtime.take_folder_dialog();
        let asked_one = asked.is_some();
        if asked_one {
            self.folder_dialog = asked;
        }
        if asked_one || self.asks.changed.get() {
            self.wake_now();
        }
    }

    /// Builds the models again if the app may have changed since they were.
    pub(super) fn read_models(&mut self) {
        if !std::mem::take(&mut self.stale) {
            return;
        }
        let models = Models::of(&self.runtime);
        if models != self.models {
            self.models = models;
            self.unseen = true;
        }
    }

    pub(super) fn wake_now(&mut self) {
        // A full channel already has a wake waiting.
        let _ = self.wake.try_send(());
    }
}
