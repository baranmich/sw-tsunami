//! The fork's macOS shell (`brand.toml` `[ui] shell = "mac"`): instead of upstream's Word-style
//! ribbon, a compact toolbar in the title bar beside the traffic lights, an inspector on the
//! right (Text / Page / Document) and the system menu bar. Every control runs the same commands
//! as the ribbon (`app.run(id, params)`), so features, undo, the command search, macros, the
//! control channel and MCP are untouched.
//!
//! Fork-owned: upstream code only calls [`on`], [`document_frame`], [`empty_frame`] and the start
//! window (from `WordApp::ui`), which keeps upstream merges clean.

mod inspector;
pub mod menu;
pub mod start;
#[cfg(test)]
mod tests;
mod toolbar;

use egui::Ui;

use crate::WordApp;
use crate::theme::Tokens;

/// Whether this build uses the mac shell instead of the ribbon (`brand.toml` `[ui] shell`).
///
/// This crate's own tests run upstream's ribbon, so its hundreds of UI tests keep driving the
/// interface they were written for; the shell's tests turn it on with [`test_with_shell`].
pub fn on() -> bool {
    #[cfg(test)]
    {
        FORCE.with(std::cell::Cell::get)
    }
    #[cfg(not(test))]
    {
        // `WORDCRAFT_SHELL=mac|ribbon` overrides brand.toml for one run (screenshots, comparisons).
        static OVERRIDE: std::sync::OnceLock<Option<bool>> = std::sync::OnceLock::new();
        let forced = *OVERRIDE.get_or_init(|| match std::env::var("WORDCRAFT_SHELL").as_deref() {
            Ok("mac") => Some(true),
            Ok("ribbon") => Some(false),
            _ => None,
        });
        forced.unwrap_or(!cfg!(feature = "ribbon-in-tests") && wordcraft_brand::SHELL == "mac")
    }
}

#[cfg(test)]
thread_local! {
    static FORCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Test helper: run `f` with the mac shell on (this thread only).
#[cfg(test)]
pub(crate) fn test_with_shell<R>(f: impl FnOnce() -> R) -> R {
    FORCE.with(|c| c.set(true));
    let r = f();
    FORCE.with(|c| c.set(false));
    r
}

/// Per-window shell state, kept in egui's memory (not saved between launches yet).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShellState {
    pub inspector: bool,
    pub tab: InspectorTab,
}

impl Default for ShellState {
    fn default() -> Self {
        Self { inspector: true, tab: InspectorTab::Text }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InspectorTab {
    Text,
    Page,
    Document,
}

fn state_id() -> egui::Id {
    egui::Id::new("mac_shell_state")
}

pub(crate) fn state(ctx: &egui::Context) -> ShellState {
    ctx.data(|d| d.get_temp::<ShellState>(state_id())).unwrap_or_default()
}

pub(crate) fn set_state(ctx: &egui::Context, s: ShellState) {
    ctx.data_mut(|d| d.insert_temp(state_id(), s));
}

/// The window around an open document: toolbar, status bar, side panes, inspector. The caller
/// draws the page canvas in the central panel afterwards.
pub fn document_frame(app: &mut WordApp, ui: &mut Ui) {
    toolbar::show(app, ui, true);
    crate::chrome::status_bar(app, ui);
    if state(ui.ctx()).inspector {
        inspector::show(app, ui);
    }
    crate::panes::show(app, ui);
}

/// The window with no document open: the toolbar (its controls greyed out) over an empty canvas.
pub fn empty_frame(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    toolbar::show(app, ui, false);
    crate::chrome::status_bar(app, ui);
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |_| {});
}
