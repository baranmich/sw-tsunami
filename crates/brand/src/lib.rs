//! Product identity for this fork, generated at build time from the workspace's `brand.toml`.
//!
//! Upstream (WordCraft by the ArtCraft team) hard-codes its name in thousands of places. Instead of
//! editing them all, which would make every upstream merge conflict, the fork:
//! - routes interface text through [`rebrand`] at one choke point (the translation lookup), so
//!   every language's strings show [`NAME`] instead of the upstream name;
//! - uses the constants here (and [`brand_name!`]) at the few dozen other places the name reaches
//!   the user: window title, file metadata, folders, messages.
//!
//! `cargo xtask brand` lists any upstream name that reaches the user without going through either.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

include!(concat!(env!("OUT_DIR"), "/brand.rs"));

/// The name this fork replaces.
pub const UPSTREAM_NAME: &str = "WordCraft";

/// At most this many distinct strings are interned by [`rebrand`]; beyond it, text is returned
/// unchanged rather than growing without bound (interface strings are a fixed, small set).
const INTERN_CAP: usize = 4096;

/// `s` with the upstream name replaced by [`NAME`].
pub fn rebrand_owned(s: &str) -> Cow<'_, str> {
    if s.contains(UPSTREAM_NAME) { Cow::Owned(s.replace(UPSTREAM_NAME, NAME)) } else { Cow::Borrowed(s) }
}

/// `s` with the upstream name replaced by [`NAME`], as a borrowed string, for APIs that hand out
/// `&str` (the translation lookup). Each distinct string is rebranded once and kept for the life
/// of the program.
pub fn rebrand(s: &str) -> &str {
    if !s.contains(UPSTREAM_NAME) {
        return s;
    }
    static CACHE: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut map = CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(hit) = map.get(s) {
        return hit;
    }
    if map.len() >= INTERN_CAP {
        return s;
    }
    let leaked: &'static str = Box::leak(s.replace(UPSTREAM_NAME, NAME).into_boxed_str());
    map.insert(s.to_string(), leaked);
    leaked
}

/// `s` written the upstream way (the brand name back to [`UPSTREAM_NAME`]), when it names the
/// brand: the key to look up text that was rebranded before it reached the translation catalog
/// (engine status messages, errors). `None` when `s` doesn't contain the name.
pub fn upstream_form(s: &str) -> Option<String> {
    if NAME.is_empty() || !s.contains(NAME) { None } else { Some(s.replace(NAME, UPSTREAM_NAME)) }
}

/// The fork's own interface translations (`brand/i18n.tsv`): language code, English, translation.
const FORK_CATALOG: &str = include_str!("../../../brand/i18n.tsv");

/// `s` in language `code` from the fork's catalog, for text upstream's catalogs don't have.
pub fn translate(code: &str, s: &str) -> Option<&'static str> {
    static MAP: OnceLock<HashMap<(String, String), &'static str>> = OnceLock::new();
    let map = MAP.get_or_init(|| {
        FORK_CATALOG
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .filter_map(|l| {
                let mut cols = l.splitn(3, '\t');
                let (lang, src, tr) = (cols.next()?, cols.next()?, cols.next()?);
                Some(((lang.to_string(), src.to_string()), tr))
            })
            .collect()
    });
    map.get(&(code.to_string(), s.to_string())).copied()
}

/// Neutral names for third-party trademarks (`brand/terms.tsv`).
const TERMS: &str = include_str!("../../../brand/terms.tsv");

struct Term {
    lang: &'static str,
    whole: bool,
    from: &'static str,
    to: &'static str,
}

fn terms() -> &'static [Term] {
    static T: OnceLock<Vec<Term>> = OnceLock::new();
    T.get_or_init(|| {
        TERMS
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .filter_map(|l| {
                let mut c = l.splitn(4, '\t');
                let (lang, kind, from, to) = (c.next()?, c.next()?, c.next()?, c.next()?);
                (!from.is_empty()).then_some(Term { lang, whole: kind == "=", from, to })
            })
            .collect()
    })
}

/// `s` (interface text in language `code`) with third-party trademarks replaced by the neutral
/// names of `brand/terms.tsv`: whole-text rows first, then anywhere-in-text rows; the language's
/// own rows before the `*` rows. Each distinct result is made once and kept, like [`rebrand`].
pub fn neutral<'a>(code: &str, s: &'a str) -> &'a str {
    let t = terms();
    if !t.iter().any(|r| s.contains(r.from)) {
        return s;
    }
    static CACHE: OnceLock<Mutex<HashMap<(String, String), &'static str>>> = OnceLock::new();
    let mut map = CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(hit) = map.get(&(code.to_string(), s.to_string())) {
        return hit;
    }
    let order = [code, "*"];
    let whole = order.iter().find_map(|lang| t.iter().find(|r| r.whole && r.lang == *lang && r.from == s).map(|r| r.to));
    let out: String = match whole {
        Some(to) => to.to_string(),
        None => {
            let mut out = s.to_string();
            for lang in order {
                for r in t.iter().filter(|r| !r.whole && r.lang == lang) {
                    if out.contains(r.from) {
                        out = out.replace(r.from, r.to);
                    }
                }
            }
            out
        }
    };
    if out == s || map.len() >= INTERN_CAP {
        return s;
    }
    let leaked: &'static str = Box::leak(out.into_boxed_str());
    map.insert((code.to_string(), s.to_string()), leaked);
    leaked
}

/// `Some(url)` when the link is configured.
pub fn link(url: &'static str) -> Option<&'static str> {
    if url.is_empty() { None } else { Some(url) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_set_and_not_upstream() {
        assert!(!NAME.is_empty());
        assert!(!NAME.contains(UPSTREAM_NAME));
        assert_eq!(brand_name!(), NAME);
        assert_eq!(concat!(brand_name!(), " is ready"), format!("{NAME} is ready"));
    }

    #[test]
    fn rebrand_replaces_every_occurrence_and_interns() {
        let a = rebrand("Make WordCraft the default. WordCraft opens .docx.");
        assert_eq!(a, format!("Make {NAME} the default. {NAME} opens .docx."));
        let b = rebrand("Make WordCraft the default. WordCraft opens .docx.");
        assert!(std::ptr::eq(a, b), "the same string is rebranded once");
        assert_eq!(rebrand("Nothing to change"), "Nothing to change");
        assert_eq!(rebrand_owned("WordCraft User"), format!("{NAME} User"));
        assert_eq!(upstream_form(&format!("{NAME} keeps it offline")).as_deref(), Some("WordCraft keeps it offline"));
        assert_eq!(upstream_form("plain text"), None);
        assert_eq!(translate("cs", "Inspector"), Some("Inspektor"));
        assert_eq!(translate("cs", "No such text"), None);
    }

    #[test]
    fn neutral_names_replace_trademarks_in_every_language() {
        assert_eq!(neutral("en", "WordArt Styles"), "Text Art Styles");
        assert_eq!(neutral("cs", "Styly WordArt"), "Styly Text Art");
        assert_eq!(neutral("de", "SmartArt einfügen"), "Smart Graphic einfügen");
        assert_eq!(neutral("en", "Researcher"), "Research");
        assert_eq!(neutral("cs", "Výzkumník"), "Výzkumník", "translations that are already generic stay");
        assert_eq!(neutral("cs", "Výchozí Office 2003"), "Klasické");
        assert_eq!(neutral("en", "Office 2003 Default"), "Classic Default");
        assert_eq!(neutral("en", "Bold"), "Bold");
    }

    #[test]
    fn identifiers_are_stable_shapes() {
        assert!(BUNDLE_ID.contains('.'));
        assert!(!DATA_DIR.is_empty());
        assert!(SLUG.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
    }
}
