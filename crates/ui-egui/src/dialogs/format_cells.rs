use super::show::ok_cancel_buttons;
use super::*;

/// This fork: Format Cells' quick number formats, on the Number tab.
const QUICK_FORMATS: [&str; 3] = ["0", "#,##0", "0%"];

/// An entry of a one-Tab-stop list (Format Cells' tab strip and categories). Only the `stop`
/// entry (the selected one) takes keyboard focus, so Tab passes over the rest and the arrow keys
/// move within the list.
fn list_item(ui: &mut egui::Ui, selected: bool, stop: bool, text: &str) -> egui::Response {
    let sense = if stop { egui::Sense::click() } else { egui::Sense::CLICK };
    ui.add(egui::Button::selectable(selected, text).sense(sense))
}

/// The list position an arrow key moves to from `at` (of `len`), wrapping around; `back` and
/// `forward` are the keys for each way. Takes the key, and stops egui moving focus with it.
fn arrow_step(ui: &egui::Ui, back: Key, forward: Key, at: usize, len: usize) -> Option<usize> {
    let (b, f) = ui.input_mut(|i| (i.consume_key(egui::Modifiers::NONE, back), i.consume_key(egui::Modifiers::NONE, forward)));
    if len == 0 || (!b && !f) {
        return None;
    }
    ui.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
    Some(if f { (at + 1) % len } else { (at + len - 1) % len })
}

/// Draws the six-tab Format Cells editor (number, alignment, font, border, fill, protection); the Number tab shows a sample preview, and the style draft updates.
pub(super) fn format_cells(app: &mut SheetApp, ui: &mut egui::Ui, d: &mut Dialog, confirm: &mut bool) {
    // `d.tab` stays the English name (it is what `open_dialog` and tests pass); only the drawn
    // labels are translated. The Fill tab is keyed by context: the Alignment tab's "Fill" is a
    // different word in some languages.
    let lang = app.ui.language;
    const TABS: [(&str, &str); 6] = [
        ("Number", "Number"),
        ("Alignment", "Alignment"),
        ("Font", "Font"),
        ("Border", "Border"),
        ("Fill", "Format Cells|Fill"),
        ("Protection", "Protection"),
    ];
    // This fork: the whole dialog works from the keyboard. The tab strip is one Tab stop (the
    // current tab) and Left/Right switch tabs; the Number categories are one Tab stop and Up/Down
    // pick a category. Tab and Shift+Tab cycle through the dialog without leaving it: egui moves
    // focus as each widget is drawn, so Tab on the last stop and Shift+Tab on the first are taken
    // here, before any is, using last frame's ends.
    let ends_key = egui::Id::unique("formatCells.tabEnds");
    let wrap = ui.data(|m| m.get_temp::<(egui::Id, egui::Id)>(ends_key)).and_then(|(first, last)| {
        let focused = ui.memory(|m| m.focused());
        let (tab, back) = ui.input(|i| (i.key_pressed(Key::Tab) && i.modifiers.is_none(), i.key_pressed(Key::Tab) && i.modifiers.shift_only()));
        let to = if tab && focused == Some(last) {
            Some(first)
        } else if back && focused == Some(first) {
            Some(last)
        } else {
            None
        };
        if to.is_some() {
            ui.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
        }
        to
    });
    let tabs = ui
        .horizontal(|ui| {
            let mut tabs = Vec::new();
            for (tab, key) in TABS {
                let r = list_item(ui, d.tab == tab, d.tab == tab, lang.tr(key));
                if r.clicked() {
                    d.tab = tab.to_string();
                }
                tabs.push(r);
            }
            tabs
        })
        .inner;
    let current = TABS.iter().position(|(tab, _)| *tab == d.tab).unwrap_or(0);
    if tabs.get(current).is_some_and(|r| r.has_focus())
        && let Some(next) = arrow_step(ui, Key::ArrowLeft, Key::ArrowRight, current, TABS.len())
        && let (Some((tab, _)), Some(r)) = (TABS.get(next), tabs.get(next))
    {
        d.tab = tab.to_string();
        r.request_focus();
    }
    let first_stop = tabs.get(current).cloned();
    // Where the keyboard starts: the first quick format on the Number tab, else the tab strip.
    let mut start = first_stop.clone();
    ui.separator();
    let mut st: gridcraft_engine::model::Style = d.values.get("style").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    let sample = app.session.active().and_then(|doc| doc.wb.active().map(|s| s.value(doc.selection.active))).unwrap_or_default();
    match d.tab.as_str() {
        "Number" => {
            let mut code = st.num_fmt.as_str().to_string();
            // This fork: the formats used most, applied (and the dialog closed) in one press.
            let quick = ui
                .horizontal(|ui| {
                    let mut chosen = None;
                    for (i, c) in QUICK_FORMATS.into_iter().enumerate() {
                        let r =
                            ui.add(egui::Button::new(egui::RichText::new(c).monospace().size(14.0)).selected(code == c).min_size(vec2(64.0, 26.0)));
                        if i == 0 {
                            start = Some(r.clone());
                        }
                        if r.clicked() {
                            chosen = Some(c);
                        }
                    }
                    chosen
                })
                .inner;
            ui.add_space(6.0);
            ui.columns(2, |cols| {
                cols[0].label(egui::RichText::new(lang.tr("Category:")).strong());
                const CATEGORIES: [&str; 11] = [
                    "General",
                    "Number",
                    "Currency",
                    "Accounting",
                    "Short Date",
                    "Long Date",
                    "Time",
                    "Percentage",
                    "Fraction",
                    "Scientific",
                    "Text",
                ];
                let codes = CATEGORIES.map(|name| gridcraft_engine::cmd::format::format_code_for(name).to_string());
                // The selected category is the list's Tab stop (the first one when none matches).
                let selected = codes.iter().position(|c| *c == code);
                let stop = selected.unwrap_or(0);
                let mut items = Vec::new();
                for (i, name) in CATEGORIES.iter().enumerate() {
                    let r = list_item(&mut cols[0], selected == Some(i), i == stop, lang.tr(name));
                    if r.clicked()
                        && let Some(c) = codes.get(i)
                    {
                        code = c.clone();
                    }
                    items.push(r);
                }
                if items.get(stop).is_some_and(|r| r.has_focus())
                    && let Some(next) = arrow_step(&cols[0], Key::ArrowUp, Key::ArrowDown, stop, CATEGORIES.len())
                    && let (Some(c), Some(r)) = (codes.get(next), items.get(next))
                {
                    code = c.clone();
                    r.request_focus();
                }
                cols[1].label(egui::RichText::new(lang.tr("Sample")).strong());
                let preview = app.session.active().map(|doc| gridcraft_engine::display::format(&sample, &code, &doc.wb).text).unwrap_or_default();
                cols[1].label(egui::RichText::new(preview).font(theme::ui_font(15.0)));
                cols[1].add_space(8.0);
                let label = cols[1].label(lang.tr("Type (custom format code):"));
                cols[1].text_edit_singleline(&mut code).labelled_by(label.id);
                // 0, #,##0 and 0% are the quick formats above.
                for c in [
                    "0.00",
                    "#,##0.00",
                    "#,##0;[Red]-#,##0",
                    "0.00%",
                    "0.00E+00",
                    "# ?/?",
                    "m/d/yyyy",
                    "d-mmm-yy",
                    "mmmm d, yyyy",
                    "h:mm AM/PM",
                    "h:mm:ss",
                    "[h]:mm:ss",
                    "@",
                ] {
                    if cols[1].small_button(c).clicked() {
                        code = c.to_string();
                    }
                }
            });
            if let Some(c) = quick {
                code = c.to_string();
                *confirm = true;
            }
            st.num_fmt = gridcraft_engine::model::NumFmt::new(&code);
        }
        "Alignment" => {
            use gridcraft_engine::model::{HAlign, VAlign};
            ui.label(egui::RichText::new(lang.tr("Horizontal")).strong());
            ui.horizontal_wrapped(|ui| {
                for (h, n) in [
                    (HAlign::General, "General"),
                    (HAlign::Left, "Left"),
                    (HAlign::Center, "Center"),
                    (HAlign::Right, "Right"),
                    (HAlign::Fill, "Fill"),
                    (HAlign::Justify, "Justify"),
                    (HAlign::CenterAcross, "Center Across Selection"),
                    (HAlign::Distributed, "Distributed"),
                ] {
                    ui.radio_value(&mut st.align.h, h, lang.tr(n));
                }
            });
            ui.label(egui::RichText::new(lang.tr("Vertical")).strong());
            ui.horizontal_wrapped(|ui| {
                for (v, n) in [
                    (VAlign::Top, "Top"),
                    (VAlign::Center, "Center"),
                    (VAlign::Bottom, "Bottom"),
                    (VAlign::Justify, "Justify"),
                    (VAlign::Distributed, "Distributed"),
                ] {
                    ui.radio_value(&mut st.align.v, v, lang.tr(n));
                }
            });
            let mut indent = st.align.indent as f32;
            ui.add(egui::Slider::new(&mut indent, 0.0..=15.0).text(lang.tr("Indent")));
            st.align.indent = indent as u8;
            let mut rot = if st.align.rotation == 255 { 0.0 } else { st.align.rotation as f32 };
            ui.add(egui::Slider::new(&mut rot, -90.0..=90.0).text(lang.tr("Orientation (degrees)")));
            st.align.rotation = rot as i16;
            ui.checkbox(&mut st.align.wrap, lang.tr("Wrap text"));
            ui.checkbox(&mut st.align.shrink, lang.tr("Shrink to fit"));
        }
        "Font" => {
            egui::ComboBox::from_label(lang.tr("Font")).selected_text(st.font.name.clone()).show_ui(ui, |ui| {
                for f in crate::fonts::families() {
                    ui.selectable_value(&mut st.font.name, f.clone(), f.clone());
                }
            });
            ui.add(egui::Slider::new(&mut st.font.size, 6.0..=72.0).text(lang.tr("Size")));
            ui.horizontal(|ui| {
                ui.checkbox(&mut st.font.bold, lang.tr("Bold"));
                ui.checkbox(&mut st.font.italic, lang.tr("Italic"));
                ui.checkbox(&mut st.font.strike, lang.tr("Strikethrough"));
            });
            use gridcraft_engine::model::{Underline, VertAlign};
            ui.horizontal(|ui| {
                for (u, n) in [
                    (Underline::None, "None"),
                    (Underline::Single, "Single"),
                    (Underline::Double, "Double"),
                    (Underline::SingleAccounting, "Single Accounting"),
                    (Underline::DoubleAccounting, "Double Accounting"),
                ] {
                    ui.radio_value(&mut st.font.underline, u, lang.tr(n));
                }
            });
            ui.horizontal(|ui| {
                ui.radio_value(&mut st.font.vert, VertAlign::Baseline, lang.tr("Normal"));
                ui.radio_value(&mut st.font.vert, VertAlign::Superscript, lang.tr("Superscript"));
                ui.radio_value(&mut st.font.vert, VertAlign::Subscript, lang.tr("Subscript"));
            });
            ui.label(lang.tr("Color:"));
            let colors = app.session.active().map(|doc| doc.wb.theme.colors).unwrap_or_default();
            if let Some(c) = crate::widgets::color_palette(ui, &colors, lang.tr("Automatic")) {
                st.font.color =
                    if c == "none" { gridcraft_engine::model::Color::Auto } else { gridcraft_engine::model::Color::from_hex(&c).unwrap_or_default() };
            }
        }
        "Border" => {
            use gridcraft_engine::model::{BorderLine, BorderStyle};
            let line = BorderLine { style: BorderStyle::Thin, color: gridcraft_engine::model::Color::Auto };
            ui.horizontal(|ui| {
                if ui.button(lang.tr("None")).clicked() {
                    st.borders = Default::default();
                }
                if ui.button(lang.tr("Outline")).clicked() {
                    st.borders.top = line;
                    st.borders.bottom = line;
                    st.borders.left = line;
                    st.borders.right = line;
                }
            });
            for (n, slot) in [
                ("Top", &mut st.borders.top),
                ("Bottom", &mut st.borders.bottom),
                ("Left", &mut st.borders.left),
                ("Right", &mut st.borders.right),
                ("Diagonal ↘", &mut st.borders.diag_down),
                ("Diagonal ↗", &mut st.borders.diag_up),
            ] {
                let n = lang.tr(n);
                ui.horizontal(|ui| {
                    ui.label(format!("{n}:"));
                    for (s, label) in [
                        (BorderStyle::None, "none"),
                        (BorderStyle::Thin, "thin"),
                        (BorderStyle::Medium, "medium"),
                        (BorderStyle::Thick, "thick"),
                        (BorderStyle::Dashed, "dashed"),
                        (BorderStyle::Dotted, "dotted"),
                        (BorderStyle::Double, "double"),
                    ] {
                        let label = lang.tr(label);
                        let response = ui.selectable_label(slot.style == s, label);
                        let selected = slot.style == s;
                        response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), selected, format!("{n} {label}")));
                        if response.clicked() {
                            slot.style = s;
                        }
                    }
                });
            }
            let theme = app.session.active().map(|doc| &doc.wb.theme).cloned().unwrap_or_default();
            crate::border_preview::sample(ui, &st.borders, &theme);
        }
        "Fill" => {
            let colors = app.session.active().map(|doc| doc.wb.theme.colors).unwrap_or_default();
            ui.label(lang.tr("Background Color:"));
            if let Some(c) = crate::widgets::color_palette(ui, &colors, lang.tr("No Color")) {
                st.fill = if c == "none" {
                    Default::default()
                } else {
                    gridcraft_engine::model::Fill::solid(gridcraft_engine::model::Color::from_hex(&c).unwrap_or_default())
                };
            }
        }
        "Protection" => {
            ui.checkbox(&mut st.protection.locked, lang.tr("Locked"));
            ui.checkbox(&mut st.protection.hidden, lang.tr("Hidden"));
            ui.label(
                egui::RichText::new(
                    lang.tr("Locking cells or hiding formulas has no effect until you protect the worksheet (Review › Protect Sheet)."),
                )
                .small(),
            );
        }
        _ => {}
    }
    if let Ok(v) = serde_json::to_value(&st) {
        d.values.insert("style".into(), v);
    }
    let mut open = true;
    let (_, cancel) = ok_cancel_buttons(ui, confirm, &mut open);
    // Tab from Cancel wraps to the tab strip, Shift+Tab from the tab strip to Cancel.
    if let Some(first) = &first_stop {
        if let Some(to) = wrap {
            ui.memory_mut(|m| m.request_focus(to));
        }
        ui.data_mut(|m| m.insert_temp(ends_key, (first.id, cancel.id)));
    }
    // A ring round the control with keyboard focus, so it can be followed on every kind of control.
    let window = ui.min_rect().expand(8.0);
    if let Some(r) = ui.memory(|m| m.focused()).and_then(|id| ui.ctx().read_response(id)).filter(|r| window.contains_rect(r.rect)) {
        let accent = crate::theme::Tokens::get(ui.ctx()).accent;
        ui.painter().rect_stroke(r.rect.expand(2.0), 3.0, egui::Stroke::new(1.5, accent), egui::StrokeKind::Outside);
    }
    if !d.focus_initialized
        && let Some(r) = &start
    {
        if r.has_focus() {
            d.focus_initialized = true;
        } else {
            r.request_focus();
        }
    }
    if *confirm && let Some(style) = d.values.get("style").cloned() {
        app.run_or_alert("home.formatCells", json!({"style": style}));
    }
    if !open {
        app.dialog = None;
        *confirm = true; // closes
    }
}
