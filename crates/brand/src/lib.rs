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
    }

    #[test]
    fn identifiers_are_stable_shapes() {
        assert!(BUNDLE_ID.contains('.'));
        assert!(!DATA_DIR.is_empty());
        assert!(SLUG.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
    }
}
