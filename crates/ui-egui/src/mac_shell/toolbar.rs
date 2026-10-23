//! The mac shell's toolbar: one compact row in the title-bar band beside the traffic lights, as
//! in Safari or Notes. Left: the outline toggle and the document's name. Right: paragraph style,
//! character formatting, lists, insert menus, comment and the inspector toggle.

use egui::{Color32, Sense, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::WordApp;
use crate::chrome::MacTitleBar;
use crate::ribbon::mi;
use crate::theme::{Tokens, medium, regular};
use crate::widgets::{menu_button, small};

/// Height of a toolbar control; leaves a margin in the 28 pt title-bar band.
const CONTROL_H: f32 = 22.0;

pub(super) fn show(app: &mut WordApp, ui: &mut Ui, doc: bool) {
    let t = Tokens::get(ui.ctx());
    // Off macOS (development and screenshots) the window has no traffic lights of its own: lay the
    // bar out as on macOS and draw stand-ins, so what you see here is what a Mac shows.
    let preview = !app.integrated_titlebar;
    let bar = if preview { MacTitleBar::default() } else { app.mac_titlebar };
    egui::Panel::top("mac_toolbar")
        .exact_size(bar.height.max(CONTROL_H + 6.0))
        .frame(egui::Frame::NONE.fill(t.title_bar).inner_margin(egui::Margin { left: bar.content_left as i8, right: 10, top: 0, bottom: 0 }))
        .show(ui, |ui| {
            let full = ui.max_rect();
            // The bar moves the window, as the title bar it replaces did.
            let drag = ui.interact(full, ui.id().with("mac_toolbar_drag"), Sense::click_and_drag());
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
            }
            if preview {
                // The panel's left margin is where the lights go: paint outside the content clip.
                let band = egui::Rect::from_min_max(egui::pos2(full.min.x - bar.content_left, full.min.y), full.max);
                stand_in_lights(&egui::Painter::new(ui.ctx().clone(), ui.layer_id(), band), band);
            }
            ui.painter().hline(full.x_range(), full.max.y - 0.5, egui::Stroke::new(1.0, t.border));
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                // Without a document (start window, empty window) the bar is just the name.
                if doc {
                    let nav = app.session.view.nav_pane;
                    small(ui, app, "navPane", None, "Navigation Pane", "view.navigationPane", json!({"value": !nav}), nav);
                    ui.add_space(6.0);
                }
                title(app, ui, &t);
                if !doc {
                    return;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                    inspector_toggle(app, ui);
                    ui.add_space(8.0);
                    {
                        // Right to left: the last group first.
                        small(ui, app, "comment", None, "New Comment", "review.newComment", json!({}), false);
                        separator(ui, &t);
                        insert_menus(app, ui);
                        separator(ui, &t);
                        let st = app.session.run("format.state", &json!({})).unwrap_or_default();
                        let flag = |k: &str| st.get(k).and_then(Value::as_bool).unwrap_or(false);
                        let lists = list_state(app);
                        small(ui, app, "numbering", None, "Numbering", "para.numbering", json!({}), lists.1);
                        small(ui, app, "bullets", None, "Bullets", "para.bullets", json!({}), lists.0);
                        separator(ui, &t);
                        small(ui, app, "underline", None, "Underline", "format.underline", json!({}), flag("underline"));
                        small(ui, app, "italic", None, "Italic", "format.italic", json!({}), flag("italic"));
                        small(ui, app, "bold", None, "Bold", "format.bold", json!({}), flag("bold"));
                        ui.add_space(4.0);
                        style_popup(app, ui, &st);
                    }
                });
            });
        });
}

/// The document's name, and "Edited" while it has unsaved changes (the macOS title convention).
fn title(app: &WordApp, ui: &mut Ui, t: &Tokens) {
    if !app.has_document() {
        ui.label(egui::RichText::new(wordcraft_brand::NAME).font(medium(13.0)).color(t.text));
        return;
    }
    ui.add(egui::Label::new(egui::RichText::new(app.title_stem()).font(medium(13.0)).color(t.text)).selectable(false).truncate());
    if app.session.dirty {
        ui.add_space(4.0);
        ui.add(egui::Label::new(egui::RichText::new(tl!("Edited")).font(regular(12.0)).color(t.text_dim)).selectable(false));
    }
}

/// The paragraph style at the caret, as a pop-up of the document's quick styles.
fn style_popup(app: &mut WordApp, ui: &mut Ui, st: &Value) {
    let current = st.get("style").and_then(Value::as_str).unwrap_or("Normal").to_string();
    let styles: Vec<(String, String)> = app.session.doc.styles.gallery().iter().map(|s| (s.id.clone(), crate::i18n::style_name(s))).collect();
    let shown = styles.iter().find(|(id, _)| *id == current).map_or_else(|| current.clone(), |(_, n)| n.clone());
    let mut pick = None;
    egui::ComboBox::from_id_salt("mac_style").width(140.0).height(400.0).selected_text(egui::RichText::new(shown).font(regular(12.0))).show_ui(
        ui,
        |ui| {
            for (id, name) in &styles {
                if ui.selectable_label(*id == current, name).clicked() {
                    pick = Some(id.clone());
                }
            }
            ui.separator();
            if ui.button(tl!("Styles Pane")).clicked() {
                pick = Some(String::new());
            }
        },
    );
    match pick {
        Some(id) if id.is_empty() => {
            let _ = app.run("styles.pane", json!({}));
        }
        Some(id) => {
            let _ = app.run("para.style", json!({"style": id}));
        }
        None => {}
    }
}

/// Insert: table, picture, shape and the rest of what the ribbon's Insert tab offers most.
fn insert_menus(app: &mut WordApp, ui: &mut Ui) {
    menu_button(ui, app, "plus", None, "Insert", false, "", |ui, app| {
        mi(ui, app, "Text Box", "insert.textBox", json!({}));
        mi(ui, app, "Link…", "ui.dialog", json!({"name": "link"}));
        mi(ui, app, "Page Break", "insert.pageBreak", json!({}));
        mi(ui, app, "Footnote", "references.footnote", json!({}));
        mi(ui, app, "Equation", "insert.equation", json!({}));
        mi(ui, app, "Symbol", "insert.symbol", json!({}));
        ui.separator();
        mi(ui, app, "Header", "insert.header", json!({}));
        mi(ui, app, "Footer", "insert.footer", json!({}));
        mi(ui, app, "Page Number", "insert.pageNumber", json!({}));
        mi(ui, app, "Table of Contents", "references.toc", json!({}));
    });
    menu_button(ui, app, "shapes", None, "Shapes", false, "insert.shape", |ui, app| {
        for (label, kind) in [
            ("Rectangle", "rectangle"),
            ("Rounded Rectangle", "roundedRectangle"),
            ("Oval", "ellipse"),
            ("Triangle", "triangle"),
            ("Line", "line"),
            ("Arrow", "arrow"),
            ("Star", "star"),
        ] {
            mi(ui, app, label, "insert.shape", json!({"kind": kind}));
        }
    });
    small(ui, app, "picture", None, "Pictures", "insert.picture", json!({}), false);
    menu_button(ui, app, "table", None, "Table", false, "insert.table", |ui, app| {
        for (label, rows, cols) in [("2 × 2", 2, 2), ("3 × 3", 3, 3), ("4 × 4", 4, 4)] {
            mi(ui, app, label, "insert.table", json!({"rows": rows, "cols": cols}));
        }
        mi(ui, app, "Insert Table…", "ui.dialog", json!({"name": "insertTable"}));
    });
}

fn inspector_toggle(_app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let mut s = super::state(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(26.0, CONTROL_H), Sense::click());
    if s.inspector {
        ui.painter().rect_filled(r, 5.0, t.checked);
    } else if resp.hovered() {
        ui.painter().rect_filled(r, 5.0, t.hover);
    }
    // A window with a panel on its right: the inspector's own glyph.
    let g = egui::Rect::from_center_size(r.center(), vec2(15.0, 12.0));
    let stroke = egui::Stroke::new(1.3, if s.inspector { t.accent } else { t.icon });
    ui.painter().rect_stroke(g, 2.0, stroke, egui::StrokeKind::Inside);
    ui.painter().vline(g.max.x - 5.0, g.y_range(), stroke);
    let resp = resp.on_hover_text(tl!("Inspector"));
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, s.inspector, tl!("Inspector")));
    if resp.clicked() {
        s.inspector = !s.inspector;
        super::set_state(ui.ctx(), s);
    }
}

fn separator(ui: &mut Ui, t: &Tokens) {
    let (r, _) = ui.allocate_exact_size(vec2(9.0, CONTROL_H), Sense::hover());
    ui.painter().vline(r.center().x, r.y_range().shrink(4.0), egui::Stroke::new(1.0, t.border));
}

/// Whether the caret's paragraph is in a bulleted or a numbered list (the toolbar's checked
/// states), as the ribbon's Paragraph group reads it.
fn list_state(app: &WordApp) -> (bool, bool) {
    let rp = app.session.doc.para_at(&app.session.sel.focus).map(|p| app.session.doc.styles.resolve_para(&p.props));
    let lists = wordcraft_engine::cmd::para::list_buttons(&app.session.doc, rp.as_ref().and_then(|r| r.numbering));
    (lists.bullets, lists.numbering)
}

/// Grey stand-ins for the traffic lights, where AppKit doesn't draw the real ones.
fn stand_in_lights(p: &egui::Painter, bar: egui::Rect) {
    let y = bar.center().y;
    for (i, c) in
        [Color32::from_rgb(0xfe, 0x5f, 0x57), Color32::from_rgb(0xfe, 0xbc, 0x2e), Color32::from_rgb(0x28, 0xc8, 0x40)].into_iter().enumerate()
    {
        p.circle_filled(pos2(bar.min.x + 16.0 + i as f32 * 20.0, y), 6.0, c);
    }
}
