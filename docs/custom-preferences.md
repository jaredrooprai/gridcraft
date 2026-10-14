# Custom preferences (jaredrooprai/gridcraft)

_Last updated: 2026-10-14._

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
- **macOS is the primary platform.** The keypad features are macOS-only, because other platforms
  have a real Num Lock.

## 1. Num Lock keypad mode (macOS)

Mac keyboards have no Num Lock. On an external keypad, the Num Lock key sends Clear (key code 71),
which winit reports as `KeyCode::NumLock` and egui drops. egui also gives keypad digits the same
`Key` as the number row. So these keys are read from winit before eframe sees them: on macOS the
app runs eframe's winit application through `eframe::create_native` inside its own event loop,
wrapped by `apps/gridcraft/src/keypad.rs` (`Keypad`).

**The behaviour:**

1. **Num Lock toggles a keypad mode.** While it's on, the status bar shows "Keypad Arrows", and
   the selection, fill handle and cell-editor border are drawn in **black** (not red, and not the
   usual accent colour).
2. **Keypad 4 8 6 2 move like the arrow keys.** Shift extends the selection, Command jumps to the
   edge of the data, and Command+Shift extends to it.
3. **Keypad 7 edits the active cell, like F2**, keeping its contents.
4. **Every other keypad key types as usual.** In a dialog the keypad always types digits; for
   example, 125 can be typed into Zoom's Custom box.
5. **While typing into a cell opened with 7, the mode shows as off.** The keypad types digits,
   "Keypad Arrows" disappears and the selection colour is normal. When the edit ends (Enter, Tab,
   Esc or clicking away), the mode is back on, black, with 4 8 6 2 as arrows again. A digit typed
   right after 7 must go into the cell, so 7 sets "typing" at once instead of waiting a frame.
6. **After `=`, `+` or `-` the mode stays on, so cells can be picked.** This applies whenever a
   formula is waiting for a reference: the caret follows `=`, `(`, `,`, `+`, `-`, `*`, `/`, `^`,
   `&`, `<`, `>`, `:`, `;` or `{` (`EditState::can_point`), or follows a reference just pointed
   at.
   - Keypad 4 8 6 2 insert or adjust a cell reference, even though 7 opened the cell in Edit mode,
     where arrows normally move the caret.
   - Keypad 7 types a `7` while pointing.
   - Once the formula no longer waits for a reference (for example a digit was typed), the
     keypad types again.
   - This doesn't apply in the formula bar, where keypad arrows keep moving the caret.
   - Example: 7, `=`, 4, `+`, 4, 2 builds `=A1+A2` when editing B1.
7. **Num Lock pressed mid-edit switches the keypad back to arrows.** The mode turns black, and a
   keypad arrow then commits the edit and moves, as arrows do in Enter mode. Pressing Num Lock
   again before an arrow returns to typing. If the mode was fully off when Num Lock is pressed
   mid-edit, it turns on with arrows active. The override ends when the edit ends.
8. **Fake modifier releases.** PC-style keypads send fake modifier releases before each keypad
   key, so egui thinks Shift or Command is up while it's held. The wrapper tracks each modifier
   key's own press and release and sends that state with every keypad key. Point mode in formulas
   reads the arrow key's own modifiers.

**Implementation at the time of writing:**

- **`apps/gridcraft/src/keypad.rs`:** `Shared { on, keys, ctx, typing, arrows }`.
  - `navigating() = on && (!typing || arrows)`.
  - `num_lock()` toggles `on`. While typing it flips `arrows` instead, or turns on with `arrows`
    set if the mode was off.
  - `set_typing(false)` clears `arrows`.
  - `route()` takes Num Lock, the navigation keys while navigating, and Command + keypad +/-.
- **`apps/gridcraft/src/main.rs`:**
  - `raw_input_hook` sets `keypad_arrows = navigating()` and feeds the queued keys to
    `SheetApp::keypad_input`.
  - After each frame, `ui()` calls
    `set_typing((editor.is_some() && !keypad_points()) || dialog.is_some())`.
- **`crates/ui-egui/src/lib.rs`:** `KeypadKey { Left, Right, Up, Down, Edit, Plus, Minus }`.
  - `SheetApp::keypad_points()` is true when an in-cell editor (not the formula bar) can point.
  - `keypad_input()` turns keys into egui events: arrows become arrow keys and set the editor's
    `enter_mode`, 7 becomes F2 (or the text `"7"` while pointing), and Plus/Minus become
    `Key::Plus`/`Key::Minus`.
- **`crates/ui-egui/src/theme.rs`:** `Tokens::keypad_selection()` draws the black selection.
- **`crates/ui-egui/src/tabs.rs`:** the status-bar "Keypad Arrows" label and its tooltip.
- **winit** (the exact version eframe uses) is a direct macOS dependency of `apps/gridcraft`.

**Tests:**

- In `keypad.rs`:
  - `num_lock_takes_only_the_navigation_keys`
  - `num_lock_while_typing_switches_to_arrows_until_the_edit_ends`
  - `command_keypad_plus_and_minus_are_taken_in_either_mode`
- In `crates/ui-egui/tests/interaction.rs`:
  - `num_lock_keypad_moves_like_the_arrows_and_7_edits`
  - `keypad_arrows_point_at_cells_after_an_operator_in_a_cell_opened_with_7`
  - `keypad_arrow_leaves_a_cell_opened_with_7`

Commits: `118b050`, `5894e53` (keypad part), `be39910`.

## 2. Cmd + / Cmd − insert and delete whole rows and columns

- **Whole rows selected:** Cmd+− deletes them, and Cmd++ inserts as many rows above them. No
  dialog opens.
- **Whole columns selected:** the same, with columns.
- **A plain range or the whole sheet:** the Delete Cells / Insert Cells dialog opens to ask how
  to shift.
- **Cmd++ can be pressed three ways:** Cmd+Shift+=, Cmd+= when whole rows or columns are
  selected, and Cmd + keypad +, which arrives without Shift.
- **Cmd + keypad + and − must work:** see the fake Command release in section 1.8. On macOS the
  keypad wrapper takes `NumpadAdd`/`NumpadSubtract` while Command is physically held, with Num
  Lock mode on or off, unless typing.

Where it lived: `crates/ui-egui/src/ribbon.rs`, in `shortcut()`, `whole_lines()` and
`insert_lines()`.

Tests: `cmd_plus_and_minus_insert_and_delete_whole_rows_and_columns` and
`command_keypad_plus_and_minus_insert_and_delete_whole_rows`.

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
`crates/ui-egui/src/formula_bar.rs`.

Test: `cmd_arrows_in_point_mode_jump_to_the_data_edge`.

Commit: `e2dbbaf`.

## 5. The in-cell editor keeps the cell's size

This deliberately differs from Excel. While editing in a cell, the editor doesn't grow to fit
long or multi-line text; the text scrolls inside the cell's own rectangle.

Where it lived: `in_cell_editor` in `crates/ui-egui/src/grid.rs`.

Commit: `353cd91`.

## 6. New workbooks default to `#,##0`

This deliberately differs from Excel. Numbers in new workbooks show without decimals and with
thousands separators. The `defaultNumberFormat` preference controls it, and `"General"` restores
Excel's default. Automatic date, percent and currency formats still apply when typed.

Where it lived: `crates/engine/src/cmd/edit.rs` and `crates/engine/src/lib.rs`, with the
preference also threaded through `fill.rs` and `cmd/print.rs`.

Commit: `055fd75`.

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

Mechanism: `Dialog::with_focus`. Focus is requested just before the box is drawn, not during the
window's first, invisible sizing pass, where egui drops it.

Test: `clicking_the_zoom_percentage_puts_the_cursor_in_the_custom_box`.

Commit: `7c832fa`.

## 9. Wider gap between the macOS traffic lights and AutoSave

Title-bar content starts 16 pt after the last traffic light (`MAC_LIGHTS_GAP`).

Commit: `9172f94`.

## 10. Typed function names stay in AutoComplete

Typing a whole function name, such as `=npv`, keeps it in the AutoComplete list, so Tab completes
it to `=NPV(` instead of moving to the next cell.

Commit: `bfd327d`.

## Keeping this file current

When a new preference lands on this fork, add a section here in the same shape: the behaviour,
where it lived, its tests and its commit.
