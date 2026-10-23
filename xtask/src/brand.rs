//! `cargo xtask brand` (fork): lists every place the upstream name or upstream links reach the
//! user without going through the brand layer (`crates/brand`, `brand.toml`).
//!
//! Run it after every upstream merge: new upstream strings show up here.
//!
//! What counts: string literals in production Rust code (not tests, examples or benches) that
//! contain `WordCraft`, `ArtCraft`, `getartcraft`, `discord.gg/artcraft` or `storytold`, unless
//! the literal is the argument of `tl!(…)` / `tc(…)` (the translation lookup rebrands those at
//! runtime), plus the macOS packaging templates. Known, deliberate exceptions live in
//! `brand/audit-allow.txt`, one `path: substring` per line.

use std::fs;
use std::path::Path;

const NEEDLES: &[&str] = &["WordCraft", "ArtCraft", "artcraft", "getartcraft", "storytold"];

pub fn run(root: &Path) -> Result<(), String> {
    let allow = load_allow(root)?;
    let mut hits = Vec::new();
    for dir in ["crates", "apps"] {
        walk(&root.join(dir), &mut |p| {
            if p.extension().is_some_and(|e| e == "rs") {
                scan_rust(root, p, &allow, &mut hits);
            }
        });
    }
    for f in ["packaging/macos/Info.plist.in", "packaging/macos/package.sh", "packaging/macos/entitlements.plist"] {
        scan_plain(root, &root.join(f), &allow, &mut hits);
    }
    if hits.is_empty() {
        println!("brand: no upstream name or link reaches the user outside the brand layer");
        Ok(())
    } else {
        for h in &hits {
            println!("{h}");
        }
        Err(format!(
            "{} place(s) show the upstream brand; route them through wordcraft_brand (or tl!), or add a deliberate exception to brand/audit-allow.txt",
            hits.len()
        ))
    }
}

fn load_allow(root: &Path) -> Result<Vec<(String, String)>, String> {
    let path = root.join("brand/audit-allow.txt");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once(": ").map(|(p, s)| (p.trim().to_string(), s.to_string())))
        .collect())
}

fn walk(dir: &Path, f: &mut dyn FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if !matches!(name, "target" | "tests" | "examples" | "benches") {
                walk(&p, f);
            }
        } else {
            f(&p);
        }
    }
}

fn is_test_file(rel: &str) -> bool {
    let file = rel.rsplit('/').next().unwrap_or(rel);
    file == "tests.rs" || file.starts_with("tests_") || file.starts_with("test_") || rel.contains("/tests/")
}

fn allowed(allow: &[(String, String)], rel: &str, line: &str) -> bool {
    allow.iter().any(|(p, s)| p == rel && line.contains(s.as_str()))
}

fn scan_rust(root: &Path, p: &Path, allow: &[(String, String)], hits: &mut Vec<String>) {
    let rel = p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/");
    if is_test_file(&rel) || rel.starts_with("crates/brand/") {
        return;
    }
    let Ok(text) = fs::read_to_string(p) else { return };
    let mut prev = String::new();
    for (i, line) in text.lines().enumerate() {
        let t = line.trim_start();
        // A `#[cfg(test)]` module runs to the end of the file in this codebase.
        if t.starts_with("#[cfg(test)]") {
            break;
        }
        if t.starts_with("//") {
            continue;
        }
        let in_literal = literals(line).iter().any(|lit| NEEDLES.iter().any(|n| lit.contains(n)));
        let translated = line.contains("tl!(") || line.contains("tc(") || prev.trim_end().ends_with("tl!(") || prev.trim_end().ends_with("tc(");
        if in_literal && !translated && !allowed(allow, &rel, line) {
            hits.push(format!("{rel}:{}: {}", i + 1, t.chars().take(140).collect::<String>()));
        }
        if !t.is_empty() {
            prev = line.to_string();
        }
    }
}

fn scan_plain(root: &Path, p: &Path, allow: &[(String, String)], hits: &mut Vec<String>) {
    let rel = p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/");
    let Ok(text) = fs::read_to_string(p) else { return };
    for (i, line) in text.lines().enumerate() {
        if NEEDLES.iter().any(|n| line.contains(n)) && !allowed(allow, &rel, line) {
            hits.push(format!("{rel}:{}: {}", i + 1, line.trim().chars().take(140).collect::<String>()));
        }
    }
}

/// The contents of the `"…"` string literals on one line (escape-aware; raw strings and
/// literals spanning lines count from their opening quote to the end of the line).
fn literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Option<String> = None;
    let mut chars = line.chars().peekable();
    let mut prev = ' ';
    while let Some(c) = chars.next() {
        match cur.as_mut() {
            None if c == '"' && prev != '\'' => cur = Some(String::new()),
            None => {}
            Some(s) if c == '\\' => {
                if let Some(n) = chars.next() {
                    s.push(n);
                }
            }
            Some(_) if c == '"' => out.extend(cur.take()),
            Some(s) => s.push(c),
        }
        prev = c;
    }
    out.extend(cur);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_are_found() {
        assert_eq!(literals(r#"f("a \"WordCraft\"", 'x', "b")"#), vec!["a \"WordCraft\"".to_string(), "b".to_string()]);
        assert!(literals("let x = 1; // WordCraft").is_empty());
    }
}
