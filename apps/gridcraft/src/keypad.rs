//! Num Lock keypad navigation (macOS).
//!
//! Mac keyboards have no Num Lock: an external keypad's Num Lock key sends Clear (key code 71),
//! which winit reports as `KeyCode::NumLock` and egui drops, and egui gives keypad digits the same
//! `Key` as the number row. So the keys are read here, from winit, before eframe sees them. Num
//! Lock toggles a mode in which keypad 4, 8, 6 and 2 move like the arrow keys and 7 edits the
//! active cell; the keys are swallowed and handed to the app as [`KeypadKey`]s, which
//! [`gridcraft_ui_egui::SheetApp::keypad_input`] turns into input on the next frame. Every other
//! key types as usual. While a cell is being edited (after 7) or a dialog is open, the mode is
//! off: the keypad types its digits, and the mode comes back on when the edit ends (Enter). A
//! formula waiting for a reference (after `=`, `+` or `-`) keeps it on, so the arrows point, and
//! Num Lock pressed mid-edit turns it back on, so a keypad arrow leaves the cell.
//!
//! Keypad + and - with Command held (insert and delete rows/columns) are taken here too, in
//! either mode: keypads send a fake Command release before them, so egui would see a bare + or -.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use eframe::UserEvent;
use gridcraft_ui_egui::KeypadKey;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, StartCause, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowId;

/// What the event loop and the app share: the mode, the keys waiting for the next frame (with
/// the modifier keys held), and the context to wake when one arrives.
#[derive(Default)]
pub struct Shared {
    pub on: bool,
    pub keys: Vec<(KeypadKey, egui::Modifiers)>,
    pub ctx: Option<egui::Context>,
    /// A cell is being edited or a dialog is open: the keypad types its digits there instead of
    /// moving. The app sets it after each frame; 7 sets it at once, so a digit typed right after
    /// 7 goes into the cell.
    pub typing: bool,
    /// Num Lock was pressed while typing: the keypad navigates anyway until typing ends (the app
    /// clears it then).
    pub arrows: bool,
}

impl Shared {
    /// Whether the keypad navigates now: Num Lock mode is on and nothing is being typed (or Num
    /// Lock was pressed to navigate away). The app shows the mode (status bar, selection colour)
    /// only then.
    pub fn navigating(&self) -> bool {
        self.on && (!self.typing || self.arrows)
    }

    /// Num Lock: toggles the mode, or while typing switches the keypad between typing and arrows
    /// (turning the mode on if it was off).
    fn num_lock(&mut self) {
        if !self.typing {
            self.on = !self.on;
        } else if !self.on {
            self.on = true;
            self.arrows = true;
        } else {
            self.arrows = !self.arrows;
        }
    }

    /// The app's state after a frame: whether a cell is edited or a dialog is open.
    pub fn set_typing(&mut self, typing: bool) {
        self.typing = typing;
        if !typing {
            self.arrows = false;
        }
    }
}

pub type Handle = Arc<Mutex<Shared>>;

/// Wraps eframe's winit application, taking the keypad keys out of its window events.
pub struct Keypad<A> {
    inner: A,
    shared: Handle,
    /// Keys whose press was taken, so their release is taken too (even if the mode changed).
    held: HashSet<KeyCode>,
    /// The modifier keys physically down. Keypads with their own Num Lock logic send fake
    /// modifier releases before each keypad key, so the modifier state says Shift or Command is
    /// up while it's held; the modifier keys' own press and release events stay right.
    modifiers: HashSet<KeyCode>,
}

impl<A> Keypad<A> {
    pub fn new(inner: A, shared: Handle) -> Self {
        Self { inner, shared, held: HashSet::new(), modifiers: HashSet::new() }
    }

    /// Whether `event` is taken here (then eframe doesn't see it).
    fn take(&mut self, event: &WindowEvent) -> bool {
        let WindowEvent::KeyboardInput { event: key, .. } = event else { return false };
        let PhysicalKey::Code(code) = key.physical_key else { return false };
        self.route(code, key.state == ElementState::Pressed, key.repeat)
    }

    /// Whether a key press or release is taken: Num Lock, the navigation keys in Num Lock mode,
    /// and Command + keypad +/- (unless typing).
    fn route(&mut self, code: KeyCode, pressed: bool, repeat: bool) -> bool {
        if is_modifier(code) {
            if pressed {
                self.modifiers.insert(code);
            } else {
                self.modifiers.remove(&code);
            }
        }
        if !pressed {
            return self.held.remove(&code);
        }
        let Ok(mut s) = self.shared.lock() else { return false };
        if code == KeyCode::NumLock {
            if !repeat {
                s.num_lock();
            }
        } else if let Some(k) = navigation(code).filter(|_| s.navigating()) {
            // Held arrows repeat; held 7 edits once.
            if !(repeat && k == KeypadKey::Edit) {
                s.keys.push((k, self.held_modifiers()));
            }
            if k == KeypadKey::Edit {
                s.typing = true;
            }
        } else if let Some(k) = sign(code).filter(|_| !s.typing && self.held_modifiers().command) {
            s.keys.push((k, self.held_modifiers()));
        } else {
            return false;
        }
        self.held.insert(code);
        if let Some(ctx) = &s.ctx {
            ctx.request_repaint();
        }
        true
    }
}

impl<A> Keypad<A> {
    /// The modifiers held, as egui reports them on macOS (Command is the Super key).
    fn held_modifiers(&self) -> egui::Modifiers {
        let down = |a, b| self.modifiers.contains(&a) || self.modifiers.contains(&b);
        let command = down(KeyCode::SuperLeft, KeyCode::SuperRight);
        egui::Modifiers {
            alt: down(KeyCode::AltLeft, KeyCode::AltRight),
            ctrl: down(KeyCode::ControlLeft, KeyCode::ControlRight),
            shift: down(KeyCode::ShiftLeft, KeyCode::ShiftRight),
            mac_cmd: command,
            command,
        }
    }
}

/// Keys that only modify others, tracked for [`Keypad::held_modifiers`].
fn is_modifier(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight
            | KeyCode::CapsLock
            | KeyCode::Fn
    )
}

/// The keypad keys that navigate in Num Lock mode.
fn navigation(code: KeyCode) -> Option<KeypadKey> {
    Some(match code {
        KeyCode::Numpad4 => KeypadKey::Left,
        KeyCode::Numpad6 => KeypadKey::Right,
        KeyCode::Numpad8 => KeypadKey::Up,
        KeyCode::Numpad2 => KeypadKey::Down,
        KeyCode::Numpad7 => KeypadKey::Edit,
        _ => return None,
    })
}

/// Keypad + and -, taken with Command held.
fn sign(code: KeyCode) -> Option<KeypadKey> {
    Some(match code {
        KeyCode::NumpadAdd => KeypadKey::Plus,
        KeyCode::NumpadSubtract => KeypadKey::Minus,
        _ => return None,
    })
}

impl<A: ApplicationHandler<UserEvent>> ApplicationHandler<UserEvent> for Keypad<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.inner.resumed(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if !self.take(&event) {
            self.inner.window_event(event_loop, window_id, event);
        }
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        self.inner.new_events(event_loop, cause);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        self.inner.user_event(event_loop, event);
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        self.inner.device_event(event_loop, device_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.inner.about_to_wait(event_loop);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.inner.suspended(event_loop);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.inner.exiting(event_loop);
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        self.inner.memory_warning(event_loop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_lock_takes_only_the_navigation_keys() {
        let shared = Handle::default();
        let mut k = Keypad::new((), shared.clone());
        let taken = |k: &mut Keypad<()>, code| k.route(code, true, false) & k.route(code, false, false);
        // Off: only Num Lock is taken.
        assert!(!taken(&mut k, KeyCode::Numpad4));
        assert!(taken(&mut k, KeyCode::NumLock));
        // On: the navigation keys are taken; everything else still types.
        for code in [KeyCode::KeyA, KeyCode::Digit2, KeyCode::Numpad5, KeyCode::Backspace, KeyCode::Enter, KeyCode::ShiftLeft] {
            assert!(!taken(&mut k, code), "{code:?}");
        }
        // Shift and Command held (their fake releases don't matter): the arrows carry them.
        assert!(!k.route(KeyCode::ShiftLeft, true, false));
        assert!(taken(&mut k, KeyCode::Numpad6));
        assert!(!k.route(KeyCode::ShiftLeft, false, false));
        assert!(!k.route(KeyCode::SuperLeft, true, false));
        assert!(taken(&mut k, KeyCode::Numpad2));
        assert!(!k.route(KeyCode::SuperLeft, false, false));
        // 7 edits and turns the mode off; the keypad types digits into the cell until the app
        // says the edit is over, then the mode is back on.
        assert!(shared.lock().is_ok_and(|s| s.navigating()));
        assert!(taken(&mut k, KeyCode::Numpad7));
        assert!(shared.lock().is_ok_and(|s| s.on && !s.navigating()));
        assert!(!taken(&mut k, KeyCode::Numpad2));
        assert!(!taken(&mut k, KeyCode::Numpad4));
        assert!(!taken(&mut k, KeyCode::Numpad7));
        let keys = shared.lock().map(|mut s| {
            s.typing = false;
            std::mem::take(&mut s.keys)
        });
        let cmd = egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND;
        assert_eq!(
            keys.ok(),
            Some(vec![(KeypadKey::Right, egui::Modifiers::SHIFT), (KeypadKey::Down, cmd), (KeypadKey::Edit, egui::Modifiers::NONE)])
        );
        assert!(shared.lock().is_ok_and(|s| s.navigating()));
        assert!(taken(&mut k, KeyCode::Numpad2));
        // A dialog is open: the keypad types digits into it too.
        if let Ok(mut s) = shared.lock() {
            s.typing = true;
        }
        assert!(!taken(&mut k, KeyCode::Numpad2));
        if let Ok(mut s) = shared.lock() {
            s.typing = false;
        }
        // Off again.
        assert!(taken(&mut k, KeyCode::NumLock));
        assert!(!taken(&mut k, KeyCode::Numpad2));
    }

    #[test]
    fn num_lock_while_typing_switches_to_arrows_until_the_edit_ends() {
        let shared = Handle::default();
        let mut k = Keypad::new((), shared.clone());
        let taken = |k: &mut Keypad<()>, code| k.route(code, true, false) & k.route(code, false, false);
        let navigating = || shared.lock().is_ok_and(|s| s.navigating());
        // On, then 7 edits: the keypad types.
        assert!(taken(&mut k, KeyCode::NumLock));
        assert!(taken(&mut k, KeyCode::Numpad7));
        assert!(!taken(&mut k, KeyCode::Numpad4));
        // Num Lock mid-edit: the arrows are back, and the mode stays on.
        assert!(taken(&mut k, KeyCode::NumLock));
        assert!(navigating());
        assert!(taken(&mut k, KeyCode::Numpad4));
        // Again: typing.
        assert!(taken(&mut k, KeyCode::NumLock));
        assert!(!navigating());
        assert!(taken(&mut k, KeyCode::NumLock));
        // The edit ends (an arrow left the cell): plain Num Lock mode again.
        if let Ok(mut s) = shared.lock() {
            s.set_typing(false);
            s.keys.clear();
        }
        assert!(navigating());
        // Mode off and typing in a cell: Num Lock turns the arrows on for this edit.
        assert!(taken(&mut k, KeyCode::NumLock));
        if let Ok(mut s) = shared.lock() {
            s.set_typing(true);
        }
        assert!(taken(&mut k, KeyCode::NumLock));
        assert!(navigating());
        if let Ok(mut s) = shared.lock() {
            s.set_typing(false);
        }
        assert!(navigating());
    }

    #[test]
    fn command_keypad_plus_and_minus_are_taken_in_either_mode() {
        let shared = Handle::default();
        let mut k = Keypad::new((), shared.clone());
        let taken = |k: &mut Keypad<()>, code| k.route(code, true, false) & k.route(code, false, false);
        // Without Command they type.
        assert!(!taken(&mut k, KeyCode::NumpadAdd));
        assert!(!taken(&mut k, KeyCode::NumpadSubtract));
        // Command held (its fake release before the key doesn't matter), Num Lock off or on.
        assert!(!k.route(KeyCode::SuperLeft, true, false));
        assert!(taken(&mut k, KeyCode::NumpadSubtract));
        assert!(taken(&mut k, KeyCode::NumLock));
        assert!(taken(&mut k, KeyCode::NumpadAdd));
        // Editing a cell: left to the editor.
        if let Ok(mut s) = shared.lock() {
            s.typing = true;
        }
        assert!(!taken(&mut k, KeyCode::NumpadAdd));
        let keys = shared.lock().map(|mut s| std::mem::take(&mut s.keys));
        let cmd = egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND;
        assert_eq!(keys.ok(), Some(vec![(KeypadKey::Minus, cmd), (KeypadKey::Plus, cmd)]));
    }

    #[test]
    fn only_the_navigation_keys_map() {
        assert_eq!(navigation(KeyCode::Numpad4), Some(KeypadKey::Left));
        assert_eq!(navigation(KeyCode::Numpad8), Some(KeypadKey::Up));
        assert_eq!(navigation(KeyCode::Numpad6), Some(KeypadKey::Right));
        assert_eq!(navigation(KeyCode::Numpad2), Some(KeypadKey::Down));
        assert_eq!(navigation(KeyCode::Numpad7), Some(KeypadKey::Edit));
        // The number row and the other keypad keys still type.
        assert_eq!(navigation(KeyCode::Digit4), None);
        assert_eq!(navigation(KeyCode::Numpad5), None);
    }
}
