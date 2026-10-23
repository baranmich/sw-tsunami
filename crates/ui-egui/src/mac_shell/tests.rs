//! The mac shell end to end: rendered with egui_kittest, driven by clicks, checked against the
//! document. Each test turns the shell on for its thread ([`super::test_with_shell`]).

use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::{Services, WordApp};

fn app() -> WordApp {
    let mut a = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), Services::default());
    a.run("file.new", json!({"template": "sample"})).unwrap();
    a
}

fn harness(a: WordApp) -> egui_kittest::Harness<'static, WordApp> {
    egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        a,
    )
}

#[test]
fn the_shell_replaces_the_ribbon() {
    super::test_with_shell(|| {
        let mut h = harness(app());
        h.run_steps(4);
        // The shell's own controls…
        assert!(h.query_by_label("Inspector").is_some(), "the inspector toggle");
        assert!(h.query_by_label("Paragraph Style").is_some(), "the inspector's Text tab");
        // …and none of the ribbon's tabs or its Word-style title bar.
        for gone in ["Mailings", "References", "AutoSave", "Share", "Editing"] {
            assert!(h.query_by_label(gone).is_none(), "{gone} is part of the ribbon shell");
        }
    });
}

#[test]
fn toolbar_bold_formats_the_selection() {
    super::test_with_shell(|| {
        let mut a = app();
        a.run("select.text", json!({"text": "Painters"})).unwrap();
        let mut h = harness(a);
        h.run_steps(4);
        h.get_all_by_label("Bold").next().unwrap().click();
        h.run_steps(4);
        let st = wordcraft_engine::cmd::format::state(&h.state().session);
        assert_eq!(st.get("bold"), Some(&json!(false)), "Painters was bold in the sample; the toolbar's Bold toggles it off");
    });
}

#[test]
fn inspector_tabs_switch_and_close() {
    super::test_with_shell(|| {
        let mut h = harness(app());
        h.run_steps(4);
        h.get_by_label("Page").click();
        h.run_steps(4);
        assert!(h.query_by_label("Margins").is_some(), "the Page tab");
        h.get_by_label("Document").click();
        h.run_steps(4);
        assert!(h.query_by_label("Track Changes").is_some(), "the Document tab");
        h.get_by_label("Inspector").click();
        h.run_steps(4);
        assert!(h.query_by_label("Track Changes").is_none(), "the toggle hides the inspector");
    });
}

#[test]
fn start_window_creates_a_document_from_a_template() {
    super::test_with_shell(|| {
        let mut a = app();
        assert!(a.start_screen());
        let mut h = harness(a);
        h.run_steps(4);
        assert!(h.query_by_label("Choose a Template").is_some());
        assert!(h.query_by_label("Good morning").is_none() && h.query_by_label("Good afternoon").is_none(), "not upstream's Home page");
        h.get_by_label("Letter").click();
        h.run_steps(4);
        let a = h.state();
        assert!(a.has_document() && !a.ui.backstage, "the letter opens in the editor");
    });
}

#[test]
fn ribbon_stays_for_upstream_tests() {
    // Outside test_with_shell the crate's tests see upstream's ribbon.
    assert!(!super::on());
    let mut h = harness(app());
    h.run_steps(4);
    assert!(h.query_by_label("Mailings").is_some());
}
