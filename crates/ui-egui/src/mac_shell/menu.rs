//! The mac shell's menu bar, as a platform-free model: menus, items and what each item runs.
//! The desktop app turns it into the system menu bar on macOS (`apps/wordcraft/src/mac_menu.rs`);
//! nothing here touches AppKit.
//!
//! The menus follow the macOS layout (app, File, Edit, Insert, Format, View, Tools, Window,
//! Help) and run the ribbon's commands. Everything else stays one search away in Help › All
//! Commands (the command search). Items carry no key equivalents: the app's own keyboard
//! handling keeps every shortcut exactly as it is.

use serde_json::{Value, json};

use crate::WordApp;

/// What an item runs.
pub struct Entry {
    pub label: String,
    pub cmd: &'static str,
    pub params: Value,
    /// Items shown with a check mark: whether it is on now.
    pub check: Option<fn(&WordApp, &egui::Context) -> bool>,
}

/// Items AppKit provides itself (they talk to the application object, not to the document).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum System {
    Services,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    Minimize,
    Zoom,
    BringAllToFront,
}

pub enum Node {
    /// An index into [`MenuBar::entries`].
    Item(usize),
    System(System),
    Separator,
    Sub(String, Vec<Node>),
}

pub struct MenuBar {
    pub menus: Vec<(String, Vec<Node>)>,
    pub entries: Vec<Entry>,
}

/// The inspector toggle is the shell's own state, not a command.
const INSPECTOR: &str = "macShell.inspector";

struct Builder {
    entries: Vec<Entry>,
}

impl Builder {
    fn item(&mut self, label: &str, cmd: &'static str, params: Value) -> Node {
        self.entries.push(Entry { label: tl!(label).to_string(), cmd, params, check: None });
        Node::Item(self.entries.len() - 1)
    }

    fn check(&mut self, label: &str, cmd: &'static str, params: Value, on: fn(&WordApp, &egui::Context) -> bool) -> Node {
        self.entries.push(Entry { label: tl!(label).to_string(), cmd, params, check: Some(on) });
        Node::Item(self.entries.len() - 1)
    }

    fn dialog(&mut self, label: &str, name: &str) -> Node {
        self.item(label, "ui.dialog", json!({"name": name}))
    }
}

fn sub(label: &str, items: Vec<Node>) -> Node {
    Node::Sub(tl!(label).to_string(), items)
}

fn fmt_flag(app: &WordApp, key: &str) -> bool {
    app.has_document() && wordcraft_engine::cmd::format::state(&app.session).get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// The menu bar in the interface's current language.
pub fn build() -> MenuBar {
    use Node::{Separator, System as Sys};
    let mut b = Builder { entries: Vec::new() };
    let about = format!("{} {}", tl!("About"), wordcraft_brand::NAME);
    let app_menu = vec![
        {
            b.entries.push(Entry { label: about, cmd: "ui.dialog", params: json!({"name": "about"}), check: None });
            Node::Item(b.entries.len() - 1)
        },
        Separator,
        b.item("Settings…", "file.options", json!({})),
        Separator,
        Sys(System::Services),
        Separator,
        Sys(System::Hide),
        Sys(System::HideOthers),
        Sys(System::ShowAll),
        Separator,
        Sys(System::Quit),
    ];
    let file = vec![
        b.item("New", "file.new", json!({})),
        b.item("Open…", "file.open", json!({})),
        Separator,
        b.item("Close", "file.close", json!({})),
        b.item("Save", "file.save", json!({})),
        b.item("Save As…", "file.saveAs", json!({})),
        b.item("Save as Template…", "file.saveTemplate", json!({})),
        Separator,
        b.item("Export PDF…", "file.exportPdf", json!({})),
        Separator,
        b.dialog("Page Setup…", "pageSetup"),
        b.item("Print…", "file.print", json!({})),
        Separator,
        b.item("Properties", "file.properties", json!({})),
    ];
    let edit = vec![
        b.item("Undo", "edit.undo", json!({})),
        b.item("Redo", "edit.redo", json!({})),
        Separator,
        b.item("Cut", "edit.cut", json!({})),
        b.item("Copy", "edit.copy", json!({})),
        b.item("Paste", "edit.paste", json!({})),
        b.item("Paste and Match Style", "edit.pasteText", json!({})),
        b.item("Select All", "select.all", json!({})),
        Separator,
        b.item("Find…", "edit.find", json!({})),
        b.item("Replace…", "edit.replace", json!({})),
        Separator,
        b.item("Spelling & Grammar", "review.spelling", json!({})),
        b.item("Thesaurus", "review.thesaurus", json!({})),
    ];
    let insert = vec![
        b.item("Page Break", "insert.pageBreak", json!({})),
        b.item("Section Break", "layout.break", json!({"kind": "nextPage"})),
        Separator,
        b.dialog("Table…", "insertTable"),
        b.item("Pictures", "insert.picture", json!({})),
        sub(
            "Shapes",
            vec![
                b.item("Rectangle", "insert.shape", json!({"kind": "rectangle"})),
                b.item("Rounded Rectangle", "insert.shape", json!({"kind": "roundedRectangle"})),
                b.item("Oval", "insert.shape", json!({"kind": "ellipse"})),
                b.item("Triangle", "insert.shape", json!({"kind": "triangle"})),
                b.item("Line", "insert.shape", json!({"kind": "line"})),
                b.item("Arrow", "insert.shape", json!({"kind": "arrow"})),
                b.item("Star", "insert.shape", json!({"kind": "star"})),
            ],
        ),
        b.item("Text Box", "insert.textBox", json!({})),
        b.item("Chart", "insert.chart", json!({})),
        b.item("Equation", "insert.equation", json!({})),
        b.item("Symbol", "insert.symbol", json!({})),
        b.item("Date & Time", "insert.dateTime", json!({})),
        Separator,
        b.dialog("Link…", "link"),
        b.dialog("Bookmark…", "bookmark"),
        b.item("Footnote", "references.footnote", json!({})),
        b.item("Endnote", "references.endnote", json!({})),
        b.item("New Comment", "review.newComment", json!({})),
        Separator,
        b.item("Header", "insert.header", json!({})),
        b.item("Footer", "insert.footer", json!({})),
        b.item("Page Number", "insert.pageNumber", json!({})),
        b.item("Table of Contents", "references.toc", json!({})),
        b.item("Cover Page", "insert.coverPage", json!({})),
    ];
    let format = vec![
        b.item("Font…", "format.fontDialog", json!({})),
        b.item("Paragraph…", "para.dialog", json!({})),
        Separator,
        b.check("Bold", "format.bold", json!({}), |a, _| fmt_flag(a, "bold")),
        b.check("Italic", "format.italic", json!({}), |a, _| fmt_flag(a, "italic")),
        b.check("Underline", "format.underline", json!({}), |a, _| fmt_flag(a, "underline")),
        b.check("Strikethrough", "format.strikethrough", json!({}), |a, _| fmt_flag(a, "strike")),
        b.item("Clear All Formatting", "format.clear", json!({})),
        Separator,
        sub(
            "Alignment",
            vec![
                b.item("Align Left", "para.alignLeft", json!({})),
                b.item("Center", "para.alignCenter", json!({})),
                b.item("Align Right", "para.alignRight", json!({})),
                b.item("Justify", "para.justify", json!({})),
            ],
        ),
        sub("Lists", vec![b.item("Bullets", "para.bullets", json!({})), b.item("Numbering", "para.numbering", json!({}))]),
        b.item("Styles Pane", "styles.pane", json!({})),
        Separator,
        sub(
            "Page Setup",
            vec![
                b.item("Portrait", "layout.orientation", json!({"value": "portrait"})),
                b.item("Landscape", "layout.orientation", json!({"value": "landscape"})),
                Separator,
                b.item("Normal Margins", "layout.margins", json!({"preset": "normal"})),
                b.item("Narrow Margins", "layout.margins", json!({"preset": "narrow"})),
                b.item("Wide Margins", "layout.margins", json!({"preset": "wide"})),
                Separator,
                b.dialog("Columns…", "columns"),
                b.dialog("Page Setup…", "pageSetup"),
            ],
        ),
        b.item("Watermark", "design.watermark", json!({})),
    ];
    // The inspector exists only in the mac shell; the ribbon shell's menu bar has the rest.
    let mut view = Vec::new();
    if super::on() {
        view.push(b.check("Inspector", INSPECTOR, json!({}), |_, ctx| super::state(ctx).inspector));
    }
    view.extend([
        b.check("Navigation Pane", "view.navigationPane", json!({}), |a, _| a.session.view.nav_pane),
        b.check("Ruler", "view.ruler", json!({}), |a, _| a.session.view.ruler),
        b.check("Show Comments", "view.commentsPane", json!({}), |a, _| a.session.view.comments_pane),
        Separator,
        b.item("Print Layout", "view.printLayout", json!({})),
        b.item("Web Layout", "view.webLayout", json!({})),
        b.item("Draft", "view.draft", json!({})),
        b.item("Read Mode", "view.readMode", json!({})),
        b.item("Focus", "view.focus", json!({})),
        Separator,
        b.item("Zoom In", "view.zoomIn", json!({})),
        b.item("Zoom Out", "view.zoomOut", json!({})),
        b.item("Actual Size", "view.zoom100", json!({})),
        Separator,
        b.check("Dark Mode", "view.darkMode", json!({}), |a, _| a.session.view.dark_mode),
        b.item("Enter Full Screen", "view.fullScreen", json!({})),
    ]);
    let tools = vec![
        b.check("Track Changes", "review.trackChanges", json!({}), |a, _| a.session.doc.settings.track_changes),
        b.item("Accept All Changes", "review.acceptAll", json!({})),
        b.item("Reject All Changes", "review.rejectAll", json!({})),
        b.item("Compare", "review.compare", json!({})),
        Separator,
        b.dialog("Word Count", "wordCount"),
        b.item("Read Aloud", "review.readAloud", json!({})),
        b.item("Check Accessibility", "file.accessibility", json!({})),
        Separator,
        sub(
            "References",
            vec![
                b.item("Insert Citation", "references.citation", json!({})),
                b.item("Bibliography", "references.bibliography", json!({})),
                b.item("Insert Caption", "references.caption", json!({})),
                b.item("Insert Index", "references.index", json!({})),
            ],
        ),
        b.item("Mail Merge", "mailings.start", json!({})),
        b.item("Macros", "tools.macros", json!({})),
    ];
    let window = vec![Sys(System::Minimize), Sys(System::Zoom), Separator, Sys(System::BringAllToFront)];
    let help = vec![b.dialog("All Commands…", "commands"), Separator, b.dialog("About", "about")];
    let menus = vec![
        (wordcraft_brand::NAME.to_string(), app_menu),
        (tl!("File").to_string(), file),
        (tl!("Edit").to_string(), edit),
        (tl!("Insert").to_string(), insert),
        (tl!("Format").to_string(), format),
        (tl!("View").to_string(), view),
        (tl!("Tools").to_string(), tools),
        (tl!("Window").to_string(), window),
        (tl!("Help").to_string(), help),
    ];
    MenuBar { menus, entries: b.entries }
}

/// Runs the item with index `key`, as a click on the same control in the window would.
pub fn activate(app: &mut WordApp, bar: &MenuBar, key: usize, ctx: &egui::Context) {
    let Some(e) = bar.entries.get(key) else { return };
    if e.cmd == INSPECTOR {
        let mut s = super::state(ctx);
        s.inspector = !s.inspector;
        super::set_state(ctx, s);
        ctx.request_repaint();
        return;
    }
    // Toggles flip from the state shown (the view commands take an explicit value).
    let params = match (e.check, e.cmd.starts_with("view.")) {
        (Some(on), true) => json!({"value": !on(app, ctx)}),
        _ => e.params.clone(),
    };
    if let Err(err) = app.run(e.cmd, params) {
        log::info!("menu {}: {err}", e.cmd);
    }
    ctx.request_repaint();
}

/// Whether the item can run now (greyed out otherwise).
pub fn enabled(app: &WordApp, e: &Entry) -> bool {
    e.cmd == INSPECTOR || crate::widgets::enabled(app, e.cmd)
}

/// The item's check mark now, for items that have one.
pub fn checked(app: &WordApp, e: &Entry, ctx: &egui::Context) -> Option<bool> {
    e.check.map(|on| on(app, ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_menu_command_exists() {
        let app = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        let bar = build();
        for e in &bar.entries {
            if e.cmd == INSPECTOR || e.cmd.starts_with("ui.") {
                continue;
            }
            assert!(app.session.registry.get(e.cmd).is_some(), "menu item {} runs unknown command {}", e.label, e.cmd);
        }
        assert!(bar.menus.len() >= 8);
    }
}
