//! Num Lock keypad navigation (macOS).
//!
//! Mac keyboards have no Num Lock: an external keypad's Num Lock key sends Clear (key code 71),
//! which winit reports as `KeyCode::NumLock` and egui drops, and egui gives keypad digits the same
//! `Key` as the number row. So the keys are read here, from winit, before eframe sees them. Num
//! Lock toggles a mode in which keypad 4, 8, 6 and 2 move like the arrow keys and 7 edits the
//! active cell; the keys are swallowed and handed to the app as [`KeypadKey`]s, which
//! [`gridcraft_ui_egui::SheetApp::keypad_input`] turns into input on the next frame. Every other
//! key types as usual, and in a dialog the keypad types its digits.

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
    /// A dialog is open: the keypad types its digits there (Zoom's Custom box) instead of moving.
    pub dialog: bool,
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

    /// Whether a key press or release is taken: Num Lock, and the navigation keys in Num Lock
    /// mode (outside dialogs).
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
                s.on = !s.on;
            }
        } else if let Some(k) = navigation(code).filter(|_| s.on && !s.dialog) {
            // Held arrows repeat; held 7 edits once.
            if !(repeat && k == KeypadKey::Edit) {
                s.keys.push((k, self.held_modifiers()));
            }
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
        assert!(taken(&mut k, KeyCode::Numpad7));
        // Shift and Command held (their fake releases don't matter): the arrows carry them.
        assert!(!k.route(KeyCode::ShiftLeft, true, false));
        assert!(taken(&mut k, KeyCode::Numpad6));
        assert!(!k.route(KeyCode::ShiftLeft, false, false));
        assert!(!k.route(KeyCode::SuperLeft, true, false));
        assert!(taken(&mut k, KeyCode::Numpad2));
        assert!(!k.route(KeyCode::SuperLeft, false, false));
        let keys = shared.lock().map(|mut s| std::mem::take(&mut s.keys));
        let cmd = egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND;
        assert_eq!(
            keys.ok(),
            Some(vec![(KeypadKey::Edit, egui::Modifiers::NONE), (KeypadKey::Right, egui::Modifiers::SHIFT), (KeypadKey::Down, cmd)])
        );
        // A dialog is open: the keypad types digits into it.
        if let Ok(mut s) = shared.lock() {
            s.dialog = true;
        }
        assert!(!taken(&mut k, KeyCode::Numpad2));
        if let Ok(mut s) = shared.lock() {
            s.dialog = false;
        }
        // Off again.
        assert!(taken(&mut k, KeyCode::NumLock));
        assert!(!taken(&mut k, KeyCode::Numpad2));
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
