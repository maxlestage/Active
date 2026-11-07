//! The keyboard and the mouse: `Res<ButtonInput<KeyCode>>`, `Res<ButtonInput<MouseButton>>`,
//! the mouse's motion and scrolling. In a page, the canvas feeds them (it takes the keyboard
//! once clicked or focused).

use std::collections::HashSet;
use std::hash::Hash;

use super::world::Resource;
use crate::scene::math::Vec2;

/// Which buttons are down, and which went down or up this frame.
#[derive(Clone, Debug)]
pub struct ButtonInput<T: Copy + Eq + Hash + 'static> {
    pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    just_released: HashSet<T>,
}

impl<T: Copy + Eq + Hash + 'static> Default for ButtonInput<T> {
    fn default() -> Self {
        ButtonInput {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
        }
    }
}

impl<T: Copy + Eq + Hash + 'static> Resource for ButtonInput<T> {}

impl<T: Copy + Eq + Hash + 'static> ButtonInput<T> {
    /// A button goes down.
    pub fn press(&mut self, button: T) {
        if self.pressed.insert(button) {
            self.just_pressed.insert(button);
        }
    }

    /// A button goes up.
    pub fn release(&mut self, button: T) {
        if self.pressed.remove(&button) {
            self.just_released.insert(button);
        }
    }

    /// Every button goes up.
    pub fn release_all(&mut self) {
        self.just_released.extend(self.pressed.drain());
    }

    /// Whether it is down.
    pub fn pressed(&self, button: T) -> bool {
        self.pressed.contains(&button)
    }

    /// Whether one of them is down.
    pub fn any_pressed(&self, buttons: impl IntoIterator<Item = T>) -> bool {
        buttons.into_iter().any(|b| self.pressed(b))
    }

    /// Whether all of them are down.
    pub fn all_pressed(&self, buttons: impl IntoIterator<Item = T>) -> bool {
        buttons.into_iter().all(|b| self.pressed(b))
    }

    /// Whether it went down this frame.
    pub fn just_pressed(&self, button: T) -> bool {
        self.just_pressed.contains(&button)
    }

    /// Whether one of them went down this frame.
    pub fn any_just_pressed(&self, buttons: impl IntoIterator<Item = T>) -> bool {
        buttons.into_iter().any(|b| self.just_pressed(b))
    }

    /// Whether it went up this frame.
    pub fn just_released(&self, button: T) -> bool {
        self.just_released.contains(&button)
    }

    /// The buttons down.
    pub fn get_pressed(&self) -> impl Iterator<Item = &T> {
        self.pressed.iter()
    }

    /// The buttons that went down this frame.
    pub fn get_just_pressed(&self) -> impl Iterator<Item = &T> {
        self.just_pressed.iter()
    }

    /// Forgets what happened this frame (done before each frame).
    pub fn clear(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// Forgets everything.
    pub fn reset_all(&mut self) {
        self.pressed.clear();
        self.clear();
    }
}

macro_rules! key_codes {
    ($($name:ident),* $(,)?) => {
        /// A key, by its place on the keyboard (the W3C `KeyboardEvent.code`):
        /// `KeyCode::KeyW` is the key of W on a QWERTY keyboard, Z on an AZERTY one.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[allow(missing_docs)]
        pub enum KeyCode {
            $($name,)*
        }

        impl KeyCode {
            /// The key of a `KeyboardEvent.code` (`"KeyW"`, `"ArrowUp"`, `"Space"`…).
            pub fn from_code(code: &str) -> Option<KeyCode> {
                match code {
                    $(stringify!($name) => Some(KeyCode::$name),)*
                    "OSLeft" | "MetaLeft" => Some(KeyCode::SuperLeft),
                    "OSRight" | "MetaRight" => Some(KeyCode::SuperRight),
                    _ => None,
                }
            }
        }
    };
}

key_codes!(
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    SuperLeft,
    SuperRight,
    CapsLock,
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Backquote,
    Comma,
    Period,
    Slash,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadEnter,
    NumpadDecimal,
);

/// A mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MouseButton {
    /// The main one.
    Left,
    /// The secondary one.
    Right,
    /// The wheel.
    Middle,
    /// Back (on the side).
    Back,
    /// Forward (on the side).
    Forward,
    /// Another one.
    Other(u16),
}

impl MouseButton {
    /// The button of a `MouseEvent.button`.
    pub fn from_dom(button: i16) -> MouseButton {
        match button {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            3 => MouseButton::Back,
            4 => MouseButton::Forward,
            n => MouseButton::Other(n.unsigned_abs()),
        }
    }
}

/// How far the mouse moved this frame, in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AccumulatedMouseMotion {
    /// The motion (y down).
    pub delta: Vec2,
}

impl Resource for AccumulatedMouseMotion {}

/// How far the wheel turned this frame, in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AccumulatedMouseScroll {
    /// The scrolling (y up: away from the user).
    pub delta: Vec2,
}

impl Resource for AccumulatedMouseScroll {}

/// What the page sent since the last frame, applied before it (in `PreUpdate`).
#[derive(Debug, Default)]
pub(crate) struct PendingInput {
    pub(crate) keys: Vec<(KeyCode, bool)>,
    pub(crate) buttons: Vec<(MouseButton, bool)>,
    pub(crate) motion: Vec2,
    pub(crate) scroll: Vec2,
    pub(crate) cursor: Option<Option<Vec2>>,
    /// The canvas lost the focus: every key goes up.
    pub(crate) blurred: bool,
}

impl Resource for PendingInput {}

/// True when the key went down this frame: `.run_if(input_just_pressed(KeyCode::Space))`.
pub fn input_just_pressed<T: Copy + Eq + Hash + 'static>(
    button: T,
) -> impl FnMut(Option<super::Res<'_, ButtonInput<T>>>) -> bool + 'static {
    move |input: Option<super::Res<'_, ButtonInput<T>>>| {
        input.is_some_and(|i| i.just_pressed(button))
    }
}

/// True while the key is down.
pub fn input_pressed<T: Copy + Eq + Hash + 'static>(
    button: T,
) -> impl FnMut(Option<super::Res<'_, ButtonInput<T>>>) -> bool + 'static {
    move |input: Option<super::Res<'_, ButtonInput<T>>>| input.is_some_and(|i| i.pressed(button))
}
