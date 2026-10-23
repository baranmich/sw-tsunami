//! The mac shell's start window, shown instead of upstream's File › Home and File › New pages:
//! a template chooser and the recent documents, as Mac document apps open. Other File pages
//! (settings, print, export, info…) stay upstream's.

use egui::{RichText, Ui, vec2};
use serde_json::json;

use crate::WordApp;
use crate::backstage::{TEMPLATES, open_list, template_tile};
use crate::theme::{Tokens, medium, regular};

/// Whether the start window replaces the File page that's open.
pub fn handles(app: &WordApp) -> bool {
    matches!(app.ui.backstage_page.as_str(), "" | "home" | "new" | "open")
}

pub fn show(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    super::toolbar::show(app, ui, false);
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas).inner_margin(egui::Margin::symmetric(40, 28))).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(tl!("Choose a Template")).font(medium(20.0)).color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.has_document() && ui.button(tl!("Cancel")).clicked() {
                        app.ui.backstage = false;
                        app.canvas.want_focus = true;
                    }
                    if ui.button(tl!("Open…")).clicked() {
                        let _ = app.run("file.open", json!({}));
                    }
                });
            });
            ui.add_space(18.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(22.0, 18.0);
                for (label, template) in TEMPLATES {
                    template_tile(ui, app, label, template);
                }
            });
            ui.add_space(28.0);
            ui.separator();
            ui.add_space(14.0);
            open_list(app, ui);
            ui.add_space(20.0);
            ui.label(
                RichText::new(tl!("Documents open in their own format: .docx, .odt, .rtf, .md and more.")).font(regular(12.0)).color(t.text_dim),
            );
        });
    });
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) && app.has_document() {
        app.ui.backstage = false;
    }
}
