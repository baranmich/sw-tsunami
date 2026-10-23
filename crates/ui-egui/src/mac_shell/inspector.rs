//! The mac shell's inspector: a panel on the right with three tabs, as in Mac document apps.
//! - Text: paragraph style, font, size, character formatting, colour, alignment, spacing, lists.
//! - Page: paper, orientation, margins, columns, header, footer, page numbers, breaks.
//! - Document: review (track changes, comments, spelling), statistics and document tools.
//!
//! Everything runs the ribbon's commands; the dialogs it opens are upstream's.

use egui::{RichText, Ui, vec2};
use serde_json::{Value, json};

use super::InspectorTab;
use crate::WordApp;
use crate::colors::{Remember, Target};
use crate::ribbon::{columns_items, margins_items, mi, orientation_items, size_menu, underline_menu};
use crate::theme::{Tokens, medium, regular};
use crate::widgets::{menu_button, small, split};

const WIDTH: f32 = 260.0;

pub(super) fn show(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::right("mac_inspector")
        .exact_size(WIDTH)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(12, 10)).stroke(egui::Stroke::new(1.0, t.border)))
        .show(ui, |ui| {
            let mut s = super::state(ui.ctx());
            tabs(ui, &t, &mut s.tab);
            super::set_state(ui.ctx(), s);
            ui.add_space(8.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match s.tab {
                InspectorTab::Text => text(app, ui, &t),
                InspectorTab::Page => page(app, ui, &t),
                InspectorTab::Document => document(app, ui, &t),
            });
        });
}

/// A segmented control, the macOS idiom for an inspector's tabs.
fn tabs(ui: &mut Ui, t: &Tokens, tab: &mut InspectorTab) {
    let items = [(InspectorTab::Text, tl!("Text")), (InspectorTab::Page, tl!("Page")), (InspectorTab::Document, tl!("Document"))];
    let (outer, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), egui::Sense::hover());
    ui.painter().rect(outer, 6.0, t.input, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    let w = outer.width() / items.len() as f32;
    for (i, (which, label)) in items.into_iter().enumerate() {
        let r = egui::Rect::from_min_size(outer.min + vec2(i as f32 * w, 0.0), vec2(w, outer.height())).shrink(2.0);
        let resp = ui.interact(r, ui.id().with(("inspector_tab", i)), egui::Sense::click());
        let on = *tab == which;
        if on {
            ui.painter().rect_filled(r, 5.0, t.checked);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, 5.0, t.hover);
        }
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, label, if on { medium(12.0) } else { regular(12.0) }, t.text);
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, on, label));
        if resp.clicked() {
            *tab = which;
        }
    }
}

fn heading(ui: &mut Ui, t: &Tokens, text: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(tl!(text)).font(medium(11.5)).color(t.text_dim));
    ui.add_space(4.0);
}

fn row(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
        add(ui);
    });
}

fn text(app: &mut WordApp, ui: &mut Ui, t: &Tokens) {
    let st = app.session.run("format.state", &json!({})).unwrap_or_default();
    let flag = |k: &str| st.get(k).and_then(Value::as_bool).unwrap_or(false);

    heading(ui, t, "Paragraph Style");
    let current = st.get("style").and_then(Value::as_str).unwrap_or("Normal").to_string();
    let styles: Vec<(String, String)> = app.session.doc.styles.gallery().iter().map(|s| (s.id.clone(), crate::i18n::style_name(s))).collect();
    let shown = styles.iter().find(|(id, _)| *id == current).map_or_else(|| current.clone(), |(_, n)| n.clone());
    let mut pick = None;
    egui::ComboBox::from_id_salt("mac_inspector_style").width(ui.available_width()).height(400.0).selected_text(shown).show_ui(ui, |ui| {
        for (id, name) in &styles {
            if ui.selectable_label(*id == current, name).clicked() {
                pick = Some(id.clone());
            }
        }
    });
    if let Some(id) = pick {
        let _ = app.run("para.style", json!({"style": id}));
    }

    heading(ui, t, "Font");
    let font = st.get("font").and_then(Value::as_str).unwrap_or("").to_string();
    let font = app.previews.font_shown(&font);
    let fams = app.previews.families();
    let recent: Vec<String> = app.ui.recent_fonts.iter().map(|f| app.previews.font_shown(f)).collect();
    let prev = app.previews.font_preview_fn();
    if let Some(f) =
        crate::widgets::font_combo_with_recent_hint(ui, "mac_font", ui.available_width(), &font, &fams, &recent, Some(&*prev), tl!("Font"))
    {
        let _ = if f == crate::widgets::ADD_FONT {
            app.run("font.add", json!({}))
        } else if f == crate::widgets::LOAD_SYSTEM_FONTS {
            app.run("font.loadSystem", json!({}))
        } else {
            let f = app.previews.font_stored(&f);
            app.run("format.font", json!({"name": f}))
        };
    }
    ui.add_space(4.0);
    row(ui, |ui| {
        let size =
            st.get("size").and_then(Value::as_f64).map(|s| if s.fract() == 0.0 { format!("{s:.0}") } else { format!("{s}") }).unwrap_or_default();
        let sizes: Vec<String> =
            wordcraft_engine::cmd::format::SIZES.iter().map(|s| if s.fract() == 0.0 { format!("{s:.0}") } else { s.to_string() }).collect();
        if let Some(v) = crate::widgets::combo_hint(ui, "mac_size", 56.0, &size, &sizes, None, tl!("Font Size"))
            && let Ok(x) = v.trim().parse::<f64>()
        {
            let _ = app.run("format.size", json!({"size": x}));
        }
        ui.add_space(6.0);
        small(ui, app, "bold", None, "Bold", "format.bold", json!({}), flag("bold"));
        small(ui, app, "italic", None, "Italic", "format.italic", json!({}), flag("italic"));
        split(ui, app, "underline", "Underline", "format.underline", json!({}), flag("underline"), None, underline_menu);
        small(ui, app, "strike", None, "Strikethrough", "format.strikethrough", json!({}), flag("strike"));
    });
    ui.add_space(4.0);
    row(ui, |ui| {
        let fc = app.canvas.last_font_color.clone();
        let sw = wordcraft_doc::Rgb::parse(&fc).map(crate::theme::c32);
        split(ui, app, "fontcolor", "Font Color", "format.color", json!({"color": fc}), false, sw, |ui, app| {
            mi(ui, app, "Automatic", "format.color", json!({"color": "auto"}));
            let theme = app.session.doc.settings.theme_colors.clone();
            crate::colors::menu(ui, app, &theme, Target::cmd("format.color").remember(Remember::FontColor));
        });
        let hl = app.canvas.last_highlight.clone();
        split(
            ui,
            app,
            "highlight",
            "Text Highlight Color",
            "format.highlight",
            json!({"color": hl}),
            false,
            None,
            crate::mini_toolbar::highlight_menu,
        );
        small(ui, app, "subscript", None, "Subscript", "format.subscript", json!({}), flag("subscript"));
        small(ui, app, "superscript", None, "Superscript", "format.superscript", json!({}), flag("superscript"));
        small(ui, app, "clear", None, "Clear All Formatting", "format.clear", json!({}), false);
    });
    ui.add_space(4.0);
    if ui.link(tl!("Font…")).clicked() {
        let _ = app.run("format.fontDialog", json!({}));
    }

    heading(ui, t, "Paragraph");
    let rp = app.session.doc.para_at(&app.session.sel.focus).map(|p| app.session.doc.styles.resolve_para(&p.props));
    let align = rp.as_ref().map(|r| r.align.visual(r.bidi)).unwrap_or_default();
    let lists = wordcraft_engine::cmd::para::list_buttons(&app.session.doc, rp.as_ref().and_then(|r| r.numbering));
    row(ui, |ui| {
        use wordcraft_doc::Align;
        small(ui, app, "alignLeft", None, "Align Left", "para.alignLeft", json!({}), align == Align::Left);
        small(ui, app, "alignCenter", None, "Center", "para.alignCenter", json!({}), align == Align::Center);
        small(ui, app, "alignRight", None, "Align Right", "para.alignRight", json!({}), align == Align::Right);
        small(ui, app, "justify", None, "Justify", "para.justify", json!({}), align == Align::Justify);
        ui.add_space(6.0);
        small(ui, app, "outdent", None, "Decrease Indent", "para.outdent", json!({}), false);
        small(ui, app, "indentLeft", None, "Increase Indent", "para.indent", json!({}), false);
    });
    ui.add_space(4.0);
    row(ui, |ui| {
        menu_button(ui, app, "lineSpacing", Some("Spacing"), "Line and Paragraph Spacing", false, "para.lineSpacing", |ui, app| {
            for v in [1.0, 1.15, 1.5, 2.0, 2.5, 3.0] {
                mi(ui, app, &format!("{v}"), "para.lineSpacing", json!({"value": v}));
            }
            ui.separator();
            mi(ui, app, "Line Spacing Options…", "para.dialog", json!({}));
        });
    });
    heading(ui, t, "Lists");
    row(ui, |ui| {
        small(ui, app, "bullets", Some("Bullets"), "Bullets", "para.bullets", json!({}), lists.bullets);
        small(ui, app, "numbering", Some("Numbering"), "Numbering", "para.numbering", json!({}), lists.numbering);
    });
    ui.add_space(4.0);
    if ui.link(tl!("Paragraph…")).clicked() {
        let _ = app.run("para.dialog", json!({}));
    }
}

fn page(app: &mut WordApp, ui: &mut Ui, t: &Tokens) {
    heading(ui, t, "Page Setup");
    row(ui, |ui| {
        menu_button(ui, app, "size", Some("Size"), "Size", false, "layout.size", size_menu);
        menu_button(ui, app, "orientation", Some("Orientation"), "Orientation", false, "layout.orientation", orientation_items);
    });
    ui.add_space(4.0);
    row(ui, |ui| {
        menu_button(ui, app, "margins", Some("Margins"), "Margins", false, "layout.margins", margins_items);
        menu_button(ui, app, "columns", Some("Columns"), "Columns", false, "layout.columns", columns_items);
    });
    ui.add_space(4.0);
    if ui.link(tl!("Page Setup…")).clicked() {
        let _ = app.run("ui.dialog", json!({"name": "pageSetup"}));
    }
    heading(ui, t, "Header & Footer");
    row(ui, |ui| {
        small(ui, app, "header", Some("Header"), "Header", "insert.header", json!({}), false);
        small(ui, app, "footer", Some("Footer"), "Footer", "insert.footer", json!({}), false);
    });
    ui.add_space(4.0);
    row(ui, |ui| {
        small(ui, app, "pageNumber", Some("Page Number"), "Page Number", "insert.pageNumber", json!({}), false);
    });
    heading(ui, t, "Breaks");
    row(ui, |ui| {
        small(ui, app, "pageBreak", Some("Page Break"), "Page Break", "insert.pageBreak", json!({}), false);
        small(ui, app, "breaks", Some("Section Break"), "Section Break", "layout.break", json!({"kind": "nextPage"}), false);
    });
    heading(ui, t, "Background");
    row(ui, |ui| {
        menu_button(ui, app, "pageColor", Some("Page Color"), "Page Color", false, "design.pageColor", |ui, app| {
            mi(ui, app, "No Color", "design.pageColor", json!({"color": null}));
            let theme = app.session.doc.settings.theme_colors.clone();
            crate::colors::menu(ui, app, &theme, Target::cmd("design.pageColor"));
        });
        small(ui, app, "pageBorders", Some("Page Borders"), "Page Borders", "design.pageBorders", json!({}), false);
    });
}

fn document(app: &mut WordApp, ui: &mut Ui, t: &Tokens) {
    heading(ui, t, "Tracking");
    let on = app.session.doc.settings.track_changes;
    row(ui, |ui| {
        small(ui, app, "trackChanges", Some("Track Changes"), "Track Changes", "review.trackChanges", json!({}), on);
    });
    ui.add_space(4.0);
    row(ui, |ui| {
        small(ui, app, "accept", Some("Accept"), "Accept", "review.accept", json!({}), false);
        small(ui, app, "reject", Some("Reject"), "Reject", "review.reject", json!({}), false);
    });
    heading(ui, t, "Comments");
    row(ui, |ui| {
        small(ui, app, "newComment", Some("New Comment"), "New Comment", "review.newComment", json!({}), false);
    });
    ui.add_space(4.0);
    row(ui, |ui| {
        let shown = app.session.view.comments_pane;
        small(ui, app, "showComments", Some("Show Comments"), "Show Comments", "view.commentsPane", json!({"value": !shown}), shown);
    });
    heading(ui, t, "Proofing");
    row(ui, |ui| {
        small(ui, app, "spelling", Some("Spelling & Grammar"), "Spelling & Grammar", "review.spelling", json!({}), false);
    });
    ui.add_space(4.0);
    row(ui, |ui| {
        small(ui, app, "wordCount", Some("Word Count"), "Word Count", "ui.dialog", json!({"name": "wordCount"}), false);
        small(ui, app, "thesaurus", Some("Thesaurus"), "Thesaurus", "review.thesaurus", json!({}), false);
    });
    heading(ui, t, "Document");
    row(ui, |ui| {
        small(ui, app, "toc", Some("Table of Contents"), "Table of Contents", "references.toc", json!({}), false);
    });
    ui.add_space(4.0);
    row(ui, |ui| {
        small(ui, app, "properties", Some("Properties"), "Properties", "file.properties", json!({}), false);
        small(ui, app, "protect", Some("Restrict Editing"), "Restrict Editing", "review.restrict", json!({}), false);
    });
}
