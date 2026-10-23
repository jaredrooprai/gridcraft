//! The in-cell editor keeps the cell's size, Wrap Text or not: this fork's choice (Excel's
//! widens to one long line, or wraps and grows down in a Wrap Text cell). Long text scrolls inside.
use egui::{Context, Id, RawInput, Rect, pos2, vec2};
use gridcraft_engine::Session;
use gridcraft_ui_egui::SheetApp;
use serde_json::json;

const TEXT: &str = "Since the 1960s, NASA has left hardware across the solar system, on the Moon, on Mars, and in deep space.";

/// Runs a few frames of the app in a 1200 × 800 window.
fn frames(ctx: &Context, app: &mut SheetApp) {
    for _ in 0..4 {
        let mut output =
            ctx.run_ui(RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))), ..Default::default() }, |ui| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            });
        output.textures_delta.clear();
    }
}

/// The editor's text field rect while A1 (narrow, wrap on or off) is edited with [`TEXT`].
fn editor_rect(wrap: bool) -> Rect {
    let ctx = Context::default();
    let mut session = Session::new();
    session.new_workbook();
    session.execute("home.columnWidth", json!({"cols": "A:A", "chars": 12})).unwrap();
    session.execute("home.wrapText", json!({"range": "A1", "on": wrap})).unwrap();
    let mut app = SheetApp::new(session, Default::default());
    frames(&ctx, &mut app);
    app.begin_edit(Some(TEXT.into()), false);
    frames(&ctx, &mut app);
    ctx.read_response(Id::new("gridcraft.cell_editor")).expect("the in-cell editor is shown").rect
}

#[test]
fn a_long_edit_stays_in_the_cells_own_rectangle() {
    for wrap in [true, false] {
        let field = editor_rect(wrap);
        assert!(field.width() < 120.0, "wrap {wrap}: the editor keeps the column's width: {field:?}");
        assert!(field.height() < 30.0, "wrap {wrap}: and the row's height: {field:?}");
    }
}

/// A short Wrap Text edit stays one line tall: the field doesn't reserve rows over the cells below.
#[test]
fn a_short_wrap_text_edit_stays_one_line_tall() {
    let ctx = Context::default();
    let mut session = Session::new();
    session.new_workbook();
    session.execute("home.wrapText", json!({"range": "A1", "on": true})).unwrap();
    let mut app = SheetApp::new(session, Default::default());
    frames(&ctx, &mut app);
    app.begin_edit(Some("short".into()), false);
    frames(&ctx, &mut app);
    let field = ctx.read_response(Id::new("gridcraft.cell_editor")).expect("the in-cell editor is shown").rect;
    assert!(field.height() < 30.0, "a one-line edit doesn't reach over the rows below: {field:?}");
}

/// An edit with line breaks (Alt+Enter) in a cell without Wrap Text shows every line: the field's
/// one row is a minimum, it grows with the text and never scrolls.
#[test]
fn an_unwrapped_multiline_edit_shows_every_line() {
    let field = |text: &str| {
        let ctx = Context::default();
        let mut session = Session::new();
        session.new_workbook();
        let mut app = SheetApp::new(session, Default::default());
        frames(&ctx, &mut app);
        app.begin_edit(Some(text.into()), false);
        frames(&ctx, &mut app);
        ctx.read_response(Id::new("gridcraft.cell_editor")).expect("the in-cell editor is shown").rect
    };
    let one = field("one").height();
    let three = field("one\ntwo\nthree").height();
    assert!(three > 2.5 * one, "three lines are shown: {three} vs one line {one}");
    assert!(three < 3.5 * one, "and no more rows are reserved: {three} vs one line {one}");
}
