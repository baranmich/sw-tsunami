//! Fork: the system menu bar on macOS (ribbon or mac shell), built with `muda` from the
//! platform-free model in `wordcraft_ui_egui::mac_shell::menu`. Items carry no key equivalents, so the app's own
//! keyboard handling keeps every shortcut as it is; the system items (Services, Hide, Quit,
//! Minimize, Zoom…) are AppKit's own.
//!
//! [`install`] once the window exists; [`poll`] every frame runs chosen items and, a few times a
//! second, greys out and checks items to match the document.

use std::cell::RefCell;
use std::sync::mpsc::{Receiver, channel};

use muda::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use wordcraft_ui_egui::WordApp;
use wordcraft_ui_egui::mac_shell::menu::{self, MenuBar, Node, System};

/// A built item: plain, or with a check mark.
enum Built {
    Plain(MenuItem),
    Check(CheckMenuItem),
}

struct State {
    // Kept alive: AppKit holds the menus only while these exist.
    _menu: Menu,
    model: MenuBar,
    items: Vec<Option<Built>>,
    shown: Vec<(bool, Option<bool>)>,
    language: String,
    events: Receiver<MenuEvent>,
    next_sync: f64,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Builds the menu bar and makes it the application's. Main thread, after the window exists.
pub fn install(app: &WordApp, ctx: &egui::Context) {
    let (tx, rx) = channel();
    let repaint = ctx.clone();
    // Menu clicks arrive while the app is idle: wake it so `poll` runs.
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        let _ = tx.send(e);
        repaint.request_repaint();
    }));
    match build(app) {
        Ok((menu, model, items)) => {
            menu.init_for_nsapp();
            let n = items.len();
            STATE.with(|s| {
                *s.borrow_mut() = Some(State {
                    _menu: menu,
                    model,
                    items,
                    shown: vec![(true, None); n],
                    language: app.ui.language.clone(),
                    events: rx,
                    next_sync: 0.0,
                })
            });
        }
        Err(e) => log::warn!("menu bar: {e}"),
    }
}

/// Runs chosen items; keeps enabled and checked states current.
pub fn poll(app: &mut WordApp, ctx: &egui::Context) {
    let rebuild = STATE.with(|s| {
        let mut s = s.borrow_mut();
        let Some(st) = s.as_mut() else { return false };
        let mut chose = false;
        while let Ok(e) = st.events.try_recv() {
            if let Some(key) = e.id.as_ref().strip_prefix('k').and_then(|k| k.parse::<usize>().ok()) {
                menu::activate(app, &st.model, key, ctx);
                chose = true;
            }
        }
        let now = ctx.input(|i| i.time);
        if chose || now >= st.next_sync {
            st.next_sync = now + 0.25;
            for (i, e) in st.model.entries.iter().enumerate() {
                let want = (menu::enabled(app, e), menu::checked(app, e, ctx));
                let (Some(item), Some(shown)) = (st.items.get(i), st.shown.get_mut(i)) else { continue };
                if *shown == want {
                    continue;
                }
                match item {
                    Some(Built::Plain(m)) => m.set_enabled(want.0),
                    Some(Built::Check(c)) => {
                        c.set_enabled(want.0);
                        c.set_checked(want.1.unwrap_or(false));
                    }
                    None => {}
                }
                *shown = want;
            }
        }
        st.language != app.ui.language
    });
    // The interface language changed: the menus follow it.
    if rebuild {
        install(app, ctx);
    }
}

type Built3 = (Menu, MenuBar, Vec<Option<Built>>);

fn build(app: &WordApp) -> Result<Built3, muda::Error> {
    wordcraft_ui_egui::i18n::set_current(wordcraft_ui_egui::i18n::Lang::from_pref(&app.ui.language));
    let model = menu::build();
    let mut items: Vec<Option<Built>> = (0..model.entries.len()).map(|_| None).collect();
    let bar = Menu::new();
    for (i, (title, nodes)) in model.menus.iter().enumerate() {
        let sub = Submenu::new(title, true);
        fill(&sub, nodes, &model, &mut items)?;
        bar.append(&sub)?;
        if i + 2 == model.menus.len() {
            sub.set_as_windows_menu_for_nsapp();
        }
        if i + 1 == model.menus.len() {
            sub.set_as_help_menu_for_nsapp();
        }
    }
    Ok((bar, model, items))
}

fn fill(sub: &Submenu, nodes: &[Node], model: &MenuBar, items: &mut [Option<Built>]) -> Result<(), muda::Error> {
    for node in nodes {
        match node {
            Node::Separator => sub.append(&PredefinedMenuItem::separator())?,
            Node::System(s) => sub.append(&system(*s))?,
            Node::Sub(title, children) => {
                let child = Submenu::new(title, true);
                fill(&child, children, model, items)?;
                sub.append(&child)?;
            }
            Node::Item(key) => {
                let Some(e) = model.entries.get(*key) else { continue };
                let id = format!("k{key}");
                let built = if e.check.is_some() {
                    let c = CheckMenuItem::with_id(id, &e.label, true, false, None);
                    sub.append(&c)?;
                    Built::Check(c)
                } else {
                    let m = MenuItem::with_id(id, &e.label, true, None);
                    sub.append(&m)?;
                    Built::Plain(m)
                };
                if let Some(slot) = items.get_mut(*key) {
                    *slot = Some(built);
                }
            }
        }
    }
    Ok(())
}

fn system(s: System) -> impl IsMenuItem {
    match s {
        System::Services => PredefinedMenuItem::services(None),
        System::Hide => PredefinedMenuItem::hide(None),
        System::HideOthers => PredefinedMenuItem::hide_others(None),
        System::ShowAll => PredefinedMenuItem::show_all(None),
        System::Quit => PredefinedMenuItem::quit(None),
        System::Minimize => PredefinedMenuItem::minimize(None),
        System::Zoom => PredefinedMenuItem::maximize(None),
        System::BringAllToFront => PredefinedMenuItem::bring_all_to_front(None),
    }
}
