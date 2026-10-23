//! Fork: recent documents in the macOS App Sandbox (Mac App Store builds).
//!
//! Sandboxed, the app may open a file only while the system has granted it: picked in an Open or
//! Save panel, dropped on the window, or opened from Finder. The grant ends with the process, so
//! File › Recent would fail after a relaunch. A security-scoped bookmark is the supported way to
//! keep it: [`poll`] makes one for each recent document while access is still live and stores
//! them in the settings folder (inside the app's container); [`install`] resolves them at start-up
//! and keeps access open for the session, so opening and saving a recent document works as before.
//!
//! Outside the sandbox (Developer ID builds) both do nothing.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use apple_scoped_bookmarks::{Bookmark, ScopedAccess};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use wordcraft_ui_egui::WordApp;

const FILE: &str = "bookmarks.json";
/// Recent documents are a short list; this bounds the file whatever happens.
const MAX: usize = 64;

struct State {
    /// Path → bookmark bytes, as stored.
    marks: BTreeMap<String, Vec<u8>>,
    /// Live access for the session; dropping a guard ends it.
    _live: Vec<ScopedAccess>,
    /// Documents whose bookmark couldn't be made (no live access): not retried this session.
    failed: HashSet<String>,
    next_poll: f64,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn sandboxed() -> bool {
    std::env::var_os("APP_SANDBOX_CONTAINER_ID").is_some()
}

fn file() -> Option<PathBuf> {
    wordcraft_control_key::settings_dir().map(|d| d.join(FILE))
}

fn load() -> BTreeMap<String, Vec<u8>> {
    let Some(text) = file().and_then(|f| std::fs::read_to_string(f).ok()) else { return BTreeMap::new() };
    let map: BTreeMap<String, String> = serde_json::from_str(&text).unwrap_or_default();
    map.into_iter().filter_map(|(k, v)| STANDARD.decode(v).ok().map(|b| (k, b))).take(MAX).collect()
}

fn save(marks: &BTreeMap<String, Vec<u8>>) {
    let Some(f) = file() else { return };
    let map: BTreeMap<&String, String> = marks.iter().map(|(k, v)| (k, STANDARD.encode(v))).collect();
    if let Ok(text) = serde_json::to_string(&map) {
        if let Some(dir) = f.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&f, text) {
            log::warn!("bookmarks: {e}");
        }
    }
}

/// Resolves the stored bookmarks and keeps their access open. Once, at start-up.
pub fn install() {
    if !sandboxed() {
        return;
    }
    let mut marks = load();
    let mut live = Vec::new();
    let mut changed = false;
    marks.retain(|path, bytes| match Bookmark::from_bytes(bytes.clone()).resolve() {
        Ok(access) => {
            if access.is_stale()
                && let Ok(fresh) = Bookmark::create(access.path())
            {
                *bytes = fresh.into_bytes();
                changed = true;
            }
            live.push(access);
            true
        }
        Err(e) => {
            // Moved, deleted or made by another build: drop it; the document can be opened again.
            log::info!("bookmark for {path}: {e}");
            changed = true;
            false
        }
    });
    if changed {
        save(&marks);
    }
    log::info!("sandbox: {} recent document(s) reachable", live.len());
    STATE.with(|s| *s.borrow_mut() = Some(State { marks, _live: live, failed: HashSet::new(), next_poll: 0.0 }));
}

/// Bookmarks documents newly added to File › Recent while their access is live; forgets those
/// that left the list. A few times a second at most.
pub fn poll(app: &WordApp, ctx: &egui::Context) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let Some(st) = s.as_mut() else { return };
        let now = ctx.input(|i| i.time);
        if now < st.next_poll {
            return;
        }
        st.next_poll = now + 1.0;
        let mut changed = false;
        for path in app.ui.recent.iter().take(MAX) {
            if st.marks.contains_key(path) || st.failed.contains(path) {
                continue;
            }
            match Bookmark::create(path) {
                Ok(b) => {
                    st.marks.insert(path.clone(), b.into_bytes());
                    changed = true;
                }
                Err(e) => {
                    log::info!("bookmark for {path}: {e}");
                    st.failed.insert(path.clone());
                }
            }
        }
        let before = st.marks.len();
        st.marks.retain(|k, _| app.ui.recent.contains(k));
        changed |= st.marks.len() != before;
        if changed {
            save(&st.marks);
        }
    });
}
