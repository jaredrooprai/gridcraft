//! This fork: Format Cells works from the keyboard alone, and its Number tab has quick 0, #,##0
//! and 0% formats that apply in one press.

use egui::{Event, Key, Modifiers, vec2};
use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::Session;
use gridcraft_engine::core::CellRef;
use gridcraft_ui_egui::SheetApp;
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness() -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("cell.set", json!({"cell": "C3", "input": "1234.5"})).unwrap();
    session.execute("selection.set", json!({"cell": "C3"})).unwrap();
    let mut h = egui_kittest::Harness::builder().with_size(vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.run_steps(6);
    h.state_mut().open_dialog("formatCells", json!({}));
    h.run_steps(6);
    h
}

fn press(h: &mut Harness, key: Key, modifiers: Modifiers) {
    h.input_mut().events.push(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
    h.input_mut().events.push(Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers });
    h.run_steps(3);
}

fn c3_format(h: &Harness) -> String {
    let d = h.state().session.active().unwrap();
    let sheet = d.wb.active().unwrap();
    d.wb.styles.get(sheet.style_id(CellRef::parse("C3").unwrap())).num_fmt.as_str().to_string()
}

fn tab(h: &Harness) -> Option<String> {
    h.state().dialog.as_ref().map(|d| d.tab.clone())
}

#[test]
fn it_opens_on_the_first_quick_format_and_right_then_enter_applies_the_next() {
    let mut h = harness();
    press(&mut h, Key::ArrowRight, Modifiers::NONE);
    press(&mut h, Key::Enter, Modifiers::NONE);
    assert!(h.state().dialog.is_none(), "Enter applied and closed the dialog");
    assert_eq!(c3_format(&h), "#,##0");
}

#[test]
fn each_quick_format_applies_in_one_press() {
    for (presses, want) in [(0, "0"), (1, "#,##0"), (2, "0%")] {
        let mut h = harness();
        for _ in 0..presses {
            press(&mut h, Key::ArrowRight, Modifiers::NONE);
        }
        press(&mut h, Key::Space, Modifiers::NONE);
        assert!(h.state().dialog.is_none(), "{want}: one press applies and closes");
        assert_eq!(c3_format(&h), want);
    }
    // And by clicking.
    let mut h = harness();
    h.get_by_label("0%").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    assert_eq!(c3_format(&h), "0%");
}

#[test]
fn the_tab_strip_is_one_tab_stop_and_left_right_switch_tabs() {
    let mut h = harness();
    // Shift+Tab from the first quick format reaches the tab strip.
    press(&mut h, Key::Tab, Modifiers::SHIFT);
    press(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(tab(&h).as_deref(), Some("Alignment"));
    press(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(tab(&h).as_deref(), Some("Font"));
    press(&mut h, Key::ArrowLeft, Modifiers::NONE);
    press(&mut h, Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(tab(&h).as_deref(), Some("Number"));
    // Left from the first tab wraps to the last.
    press(&mut h, Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(tab(&h).as_deref(), Some("Protection"));
    // Tab leaves the strip (one stop) instead of walking every tab.
    press(&mut h, Key::ArrowRight, Modifiers::NONE);
    press(&mut h, Key::Tab, Modifiers::NONE);
    press(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(tab(&h).as_deref(), Some("Number"), "the arrow no longer switches tabs once Tab moved on");
}

#[test]
fn up_and_down_pick_a_category_and_enter_applies_it() {
    let mut h = harness();
    // The three quick formats, then the category list.
    for _ in 0..3 {
        press(&mut h, Key::Tab, Modifiers::NONE);
    }
    press(&mut h, Key::ArrowDown, Modifiers::NONE);
    press(&mut h, Key::ArrowDown, Modifiers::NONE);
    press(&mut h, Key::ArrowUp, Modifiers::NONE);
    press(&mut h, Key::Enter, Modifiers::NONE);
    assert!(h.state().dialog.is_none(), "Enter is OK");
    assert_eq!(c3_format(&h), gridcraft_engine::cmd::format::format_code_for("Number"));
}

#[test]
fn tab_cycles_through_the_dialog_without_leaving_it() {
    let mut h = harness();
    let dialog = h.ctx.memory(|m| m.area_rect(egui::Id::unique("dialog").with("formatCells"))).expect("the dialog is shown");
    let mut seen = std::collections::HashSet::new();
    for i in 0..60 {
        press(&mut h, Key::Tab, Modifiers::NONE);
        let id = h.ctx.memory(|m| m.focused()).unwrap_or_else(|| panic!("Tab {i}: something in the dialog has focus"));
        let rect = h.ctx.read_response(id).map(|r| r.rect).expect("the focused widget is shown");
        assert!(dialog.expand(1.0).contains_rect(rect), "Tab {i}: focus stays in the dialog: {rect:?} vs {dialog:?}");
        seen.insert(id);
    }
    assert!(seen.len() > 5, "Tab visits the dialog's controls, not one spot: {}", seen.len());
    // Shift+Tab goes back the other way, still inside.
    for i in 0..20 {
        press(&mut h, Key::Tab, Modifiers::SHIFT);
        let id = h.ctx.memory(|m| m.focused()).unwrap_or_else(|| panic!("Shift+Tab {i}: something has focus"));
        let rect = h.ctx.read_response(id).map(|r| r.rect).expect("the focused widget is shown");
        assert!(dialog.expand(1.0).contains_rect(rect), "Shift+Tab {i}: focus stays in the dialog");
    }
}

#[test]
fn enter_on_a_focused_cancel_cancels() {
    let mut h = harness();
    // Shift+Tab from the tab strip wraps to Cancel.
    press(&mut h, Key::Tab, Modifiers::SHIFT);
    press(&mut h, Key::Tab, Modifiers::SHIFT);
    press(&mut h, Key::Enter, Modifiers::NONE);
    assert!(h.state().dialog.is_none(), "Cancel closed the dialog");
    assert_eq!(c3_format(&h), "#,##0", "nothing was applied (this fork's default format is kept)");
}
