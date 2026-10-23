# Custom preferences (jaredrooprai/gridcraft)

_Last updated: 2026-10-23._

This fork of GridCraft (`main` on `jaredrooprai/gridcraft`; upstream is `storytold/gridcraft`)
carries behaviour changes that its owner wants kept. Some deliberately differ from Excel. This
file is the spec for re-applying them to a newer GridCraft, by merging upstream or starting from a
fresh checkout.

## For the agent doing the port

- **Port the behaviour, not the code.** Each section says what the user should see. The code
  pointers and commits below are where it lived at the time of writing and may have moved.
- **Check upstream first.** Upstream may already do some of these. Where a section differs from
  Excel on purpose, keep this fork's behaviour.
- **Every change lands with tests.** The tests named here are the acceptance checks. Rewrite them
  to fit the new code if needed, but keep what they assert.
- **Follow the repo's rules** (`CLAUDE.md`): no panics, everything is a command, the UI is thin,
  `cargo xtask ci` passes.
- **Commits are credited to the user alone:** no `Co-Authored-By` or other AI attribution.
- **Commit to `main`; ask before pushing** to the `fork` remote.
- **macOS is the primary platform.**

## 1. Num Lock keypad mode (removed)

Removed on 2026-10-23 at the user's request; don't port it. The fork once read the keypad from
winit before eframe (`apps/gridcraft/src/keypad.rs`, commits `118b050`, `5894e53`, `be39910`) so
Num Lock turned keypad 4 8 6 2 into arrows and 7 into F2. The desktop app is back to upstream's
`eframe::run_native` startup. The user now gets keypad arrows system-wide from Karabiner-Elements
instead (Control + keypad 8/4/6/2 → arrow keys, `~/.config/karabiner/`), outside this repo.

## 2. Cmd + / Cmd − insert and delete whole rows and columns

- **Whole rows selected:** Cmd+− deletes them, and Cmd++ inserts as many rows above them. No
  dialog opens.
- **Whole columns selected:** the same, with columns.
- **A plain range or the whole sheet:** the Delete Cells / Insert Cells dialog opens to ask how
  to shift.
- **Cmd++ can be pressed three ways:** Cmd+Shift+=, Cmd+= when whole rows or columns are
  selected, and Cmd + keypad +, which arrives without Shift.
- Since section 1's keypad wrapper was removed, Cmd + keypad +/− reaches the app as egui reports
  it. A PC-style keypad that sends a fake Command release before its keys may not trigger it.

Where it lived: `crates/ui-egui/src/ribbon.rs`, in `shortcut()` and `insert_lines()`.

Upstream 0.8.0 added Excel's version of this (Ctrl+Shift+= and Ctrl+− on whole rows or columns,
with `whole_lines()` covering several areas and `single_area()` sending a multi-area insert to the
dialog). The fork keeps upstream's checks and adds only two things on top: keypad + (a `Key::Plus`
without Shift) inserts, and Cmd+= inserts when whole rows or columns are selected.

Test: `cmd_plus_and_minus_insert_and_delete_whole_rows_and_columns`.

Commits: `5894e53`, `be39910`.

## 3. A leading + or − starts a formula

This matches Excel.

- **`+20+20` becomes the formula `=+20+20`**, showing 40. `-5-3` becomes a formula showing −8.
- **A plain signed number stays a constant:** `+20` and `-5`.
- **Input that doesn't parse as a formula stays text:** `+1 555 1234`.
- **Pointing works after a leading sign.** With `+`, `-`, `+20+` or `-5-` typed, an arrow or
  click inserts a reference.

Where it lived: `input_to_cell` in `crates/engine/src/cmd/edit.rs`; `EditState::is_formula` and
`can_point` in `crates/ui-egui/src/editor.rs`.

Tests: `a_leading_sign_starts_a_formula` and `a_leading_sign_points_like_equals`.

Commit: `5f326cd`.

## 4. Cmd/Ctrl + arrow in point mode jumps to the edge of the data

While pointing in a formula, Cmd+arrow (Ctrl+arrow on Windows and Linux) moves the reference to
the edge of the data, like Cmd+arrow in the grid. From a cell below `A1:A5`, it goes to `A5`,
then `A1`. A new reference, after another operator, jumps from the edited cell.

Where it lived: `point_move` in `crates/ui-egui/src/grid.rs` and the arrow handling in
`crates/ui-egui/src/formula_bar.rs`. `EditState::point_origin` (editor.rs) picks the start: the
last pointed cell only while the caret is at the end of that reference (upstream's
`point_at_caret`), else the edited cell. Upstream's Shift+arrow `point_anchor` is ignored for a
new reference for the same reason. Upstream's `pointing_along_a_frozen_row_keeps_the_edited_cell_in_view`
sets `point_cell` without a pointed reference; the fork's copy also sets `point`.

Test: `cmd_arrows_in_point_mode_jump_to_the_data_edge`.

Commit: `e2dbbaf`.

## 5. The in-cell editor keeps the cell's size

This deliberately differs from Excel. While editing in a cell, the editor doesn't grow to fit
long or multi-line text; the text scrolls inside the cell's own rectangle.

Where it lived: `in_cell_editor` in `crates/ui-egui/src/grid.rs`. The editor field is single-line
(`editor_widget(.., false, ..)`) even in a Wrap Text cell, so it scrolls sideways to keep the caret
inside the cell.

Upstream later made the editor widen to one long line and, with Wrap Text, wrap and grow down. Its
tests for that are rewritten in the fork: `one_long_line_keeps_the_editor_at_the_cells_size`
(`tests/it/in_cell_editor_size.rs`) and `a_long_edit_stays_in_the_cells_own_rectangle`
(`tests/it/in_cell_editor_wrap.rs`, where `the_editor_measures_the_text_at_the_fields_wrap_width`
was dropped).

Commit: `353cd91`.

## 6. New workbooks default to `#,##0`

This deliberately differs from Excel. Numbers in new workbooks show without decimals and with
thousands separators. The `defaultNumberFormat` preference controls it, and `"General"` restores
Excel's default. Automatic date, percent and currency formats still apply when typed.

Where it lived: `crates/engine/src/cmd/edit.rs` and `crates/engine/src/lib.rs`, with the
preference also threaded through `fill.rs` and `cmd/print.rs`.

Commit: `055fd75`.

**Upstream tests:** upstream's engine tests assume Excel's General default. A test file that types
numbers and checks their format sets `s.prefs.default_number_format = "General"` in its session
helper (as `crates/engine/src/tests.rs` and `crates/engine/tests/range_fill_formats.rs` do). Do the
same for any new upstream test that fails on `#,##0` versus `General`.

## 7. Control+R and Control+D fill right and down on macOS

As in Excel for Mac, Control+R runs `edit.fillRight` and Control+D runs `edit.fillDown`,
alongside Command+R and Command+D.

Where it lived: `crates/ui-egui/src/keymap.rs`.

Commit: `2db6e0b`.

## 8. Clicking the zoom percentage opens Zoom ready to type

Clicking the zoom percentage in the status bar does three things:

- the cursor becomes a pointing hand over it
- the Zoom dialog opens with the caret in the Custom (%) box
- the current value is selected, so typing a number and pressing Enter sets the zoom

Mechanism: upstream 0.11 redrew Zoom as its own `zoom()` body in `crates/ui-egui/src/dialogs.rs`
and added `take_first_focus`, which asks for focus every frame until the field has it and then
selects its value. The fork calls it on the Custom box's `DragValue`, with
`update_while_editing(true)` so Enter in the same frame confirms the typed number. (Before 0.11 the
fork had its own `Dialog::with_focus` doing the same.) The test opens Zoom three times.

Test: `clicking_the_zoom_percentage_puts_the_cursor_in_the_custom_box`.

Commit: `7c832fa`.

## 9. Wider gap between the macOS traffic lights and AutoSave

Title-bar content starts 16 pt after the last traffic light (`MAC_LIGHTS_GAP`).

Commit: `9172f94`.

## 10. Typed function names stay in AutoComplete

Typing a whole function name, such as `=npv`, keeps it in the AutoComplete list, so Tab completes
it to `=NPV(` instead of moving to the next cell.

Commit: `bfd327d`.

## 11. Delete only empties the cell

This deliberately differs from Excel for Mac, where Delete (Backspace) clears the cell and starts
editing it. Here Delete clears the selection's contents and the cell is not opened for editing.
Fn+Delete (forward delete) also only clears.

Where it lived: the `Key::Backspace` arm of the grid's keyboard handler in
`crates/ui-egui/src/grid.rs`.

Test: `delete_clears_the_cell_without_editing_it`.

## 12. Control+Shift+C is Format Painter, and the painter applies on click or arrow move

This deliberately differs from Excel. (From 2026-10-14 to 2026-10-19, Cmd+S was the painter and Save
had no shortcut; the user moved it back. Cmd+S is Save, as in Excel.)

- **Control+Shift+C picks up the active cell's format**, like clicking the Format Painter brush.
  Pressing it again before using it cancels. It's Control, not Command. On Windows and Linux
  egui turns Ctrl+Shift+C into Copy, so in practice it's macOS only.
- **The next cell clicked, dragged across or moved to with an arrow key takes the format.** The
  painter then ends. Shift+arrow (extending the selection) doesn't paint. Double-clicking the brush
  still keeps it on for several targets.
- **Applying on click is upstream's now** (`format_painter_apply` in `crates/ui-egui/src/grid.rs`,
  since 0.11, on mouse release over cells or headers). The fork only adds the arrow-key part: the
  grid's `mv` closure calls `format_painter_apply` after a move without Shift.

Where it lived:
- `Shortcut::FormatPainter` in `crates/ui-egui/src/keymap.rs` (the key and the toggle)
- `format_painter_apply` (called from the arrow-key `mv` closure) in `crates/ui-egui/src/grid.rs`
- the `edit.formatPainter` shortcut metadata (`Ctrl+Shift+C`) in `crates/engine/src/cmd/edit.rs`
- the brush tooltip in `crates/ui-egui/src/ribbon.rs`

Test: `control_shift_c_picks_up_the_format_for_the_next_cell_clicked_or_moved_to` (which also
checks that Cmd+S doesn't pick up the painter).

## 13. Frozen panes keep their gridlines when scrolled (fix)

This is a bug fix rather than a preference; check whether upstream has fixed it before porting.
With columns frozen, scrolling right left the frozen columns without row gridlines (they looked
white). Frozen rows lost their column gridlines when scrolling down. The cause: each pane drew its
lines out to the start of the next column or row, which past a freeze is in the scrolled pane. The
pane's far edges now come from its own last column and row.

Where it lived: `gridlines()` (used by `paint_quadrant`) in `crates/ui-egui/src/grid.rs`.

Test: `frozen_panes_keep_their_gridlines_when_scrolled`.

## 14. Cmd+2 opens Zoom

This isn't an Excel shortcut. On macOS, Cmd+2 opens the Zoom dialog the same way clicking the zoom
percentage does (section 8): the caret is in the Custom (%) box with its value selected, so typing
a number and pressing Enter sets the zoom. It's macOS only, because on Windows Ctrl+2 is Bold.
(For a few minutes on 2026-10-23 it was Option+2; the user moved it to Cmd+2.)

Where it lived: `Num2 if cmd && !s && mac` in `lookup()` in `crates/ui-egui/src/keymap.rs`
(matched by the physical key, so other layouts work too).

Test: `command_2_opens_zoom_ready_to_type`.

## 15. Format Cells works from the keyboard, with quick 0, #,##0 and 0%

This deliberately differs from Excel.

- **Quick formats.** The Number tab starts with three buttons, `0`, `#,##0` and `0%`. Pressing one
  (click, Space or Enter) applies that format to the selection and closes the dialog. They were
  dropped from the list of format-code buttons below them, so each appears once.
- **The keyboard starts on the first quick format** when the dialog opens on the Number tab
  (otherwise on the current tab). Cmd+1, Right, Enter applies `#,##0`.
- **Tab and Shift+Tab** move through every control and wrap around inside the dialog; focus
  never escapes to the ribbon or grid.
- **The tab strip is one Tab stop** (the current tab). Left/Right switch tabs, wrapping.
- **The Number categories are one Tab stop** (the selected category, or General when none
  matches). Up/Down pick a category, wrapping, and the Sample follows.
- **Elsewhere the arrow keys move focus** to the nearest control (egui's own behaviour). Sliders
  take Left/Right to change their value, text fields take them to move the caret; Tab leaves both.
- **Space or Enter presses the focused control.** Enter is also OK, except on a focused Cancel,
  where it cancels, or while a drop-down is open. The Cancel rule is in `ok_cancel_buttons`, which
  every dialog's OK/Cancel row uses.
- **A green ring** (`Tokens::accent`) marks the focused control.

Where it lived: `format_cells`, `list_item`, `arrow_step`, `QUICK_FORMATS` and
`ok_cancel_buttons` in `crates/ui-egui/src/dialogs.rs`. Unselected list entries use
`Sense::CLICK` without `FOCUSABLE`, so Tab passes over them. The Tab wrap is decided at the start
of the frame from last frame's first and last stops (egui temp data `formatCells.tabEnds`), because
egui moves focus as each widget is drawn.

Tests: `crates/ui-egui/tests/it/format_cells_keyboard.rs`.

## 16. Option+Left and Option+Right change the decimal places

This deliberately differs from Excel for Mac, where Option+Right and Option+Left go to the next and
previous sheet. On macOS, with the grid focused, Option+Left runs `home.increaseDecimal` and
Option+Right runs `home.decreaseDecimal` (`#,##0` → `#,##0.0` → `#,##0.00`). Cmd+Page Down / Page Up
(Cmd+Fn+Down / Up on a Mac keyboard) still change sheets. In a cell being edited, Option+arrow
still moves the caret by word. Windows' Alt+Shift+Right / Left (group / ungroup) are unchanged.

Where it lived: the `ArrowLeft` / `ArrowRight` `alt && !s && mac` arms of `lookup()` in
`crates/ui-egui/src/keymap.rs`.

Tests: `option_left_and_right_add_and_remove_a_decimal` (`tests/it/interaction.rs`) and the
`lookup` assertions in `windows_and_mac_variants` (`tests/it/keyboard_shortcuts.rs`).

## 17. Control+7 edits the active cell

This isn't an Excel shortcut. On macOS, with the grid focused, Control+7 opens the active cell
for editing with its contents kept and the caret at the end, like F2 or Excel for Mac's
Control+U. Both the number row's 7 and the keypad's 7 work (the keymap matches the physical key,
and egui reports keypad 7 as `Num7`). Control+Shift+7 is still the outline border.

Where it lived: `Num7 if ctrl && mac && !s && !m.command` in `lookup()` in
`crates/ui-egui/src/keymap.rs`.

Tests: `control_7_edits_the_cell_keeping_its_contents` (`tests/it/interaction.rs`) and the
`lookup` assertions in `windows_and_mac_variants` (`tests/it/keyboard_shortcuts.rs`).

## Merging upstream

Last merged: upstream main after 0.11.1 (`530a8cc8`), on 2026-10-23. Things to watch when merging:

- **Conflicts so far:** the keyboard shortcuts in `ribbon.rs` (section 2), tests appended to the end of
  `crates/ui-egui/tests/it/interaction.rs` (keep both sides; it was `tests/interaction.rs` before
  0.11), and in 0.11: the in-cell editor's size (section 5), `point_move` and `point_origin`
  (section 4), the Zoom dialog (section 8), the Format Painter on release (section 12) and the
  Format Painter tooltip in `ribbon.rs`.
- **Tooltips:** since 0.11 ribbon tooltips get their shortcut from `TOOLTIP_SHORTCUTS` in
  `widgets.rs`, so "Save (⌘S)"-style keys are gone from the i18n catalogs. `ja.rs` counts
  `pt.rs`'s keys: a stray key there fails `covers_every_key_the_portuguese_catalog_translates`.
- **Clippy on macOS:** upstream's CI runs clippy on Linux. In 0.8.0 `desktop_theme.rs` bound a
  `()` value off Linux; the fork allows `clippy::let_unit_value` on that one line for non-Linux
  builds.
- **The wasm step** of `cargo xtask ci` needs `rustup target add wasm32-unknown-unknown`.

## Keeping this file current

When a new preference lands on this fork, add a section here in the same shape: the behaviour,
where it lived, its tests and its commit.
