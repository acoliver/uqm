//! What the window system reports to the engine, in the engine's terms.

/// One event from the platform's queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostEvent {
    /// A key went down; `repeat` marks the auto-repeat of a held key.
    KeyDown {
        key: i32,
        numlock: bool,
        repeat: bool,
    },
    KeyUp {
        key: i32,
        numlock: bool,
    },
    /// Text typed while character mode is on.
    TextInput(String),
    MouseButton {
        down: bool,
    },
    JoyAxis {
        port: u32,
        axis: i32,
        value: i16,
    },
    JoyHat {
        port: u32,
        hat: i32,
        state: u8,
    },
    JoyButton {
        port: u32,
        button: i32,
        down: bool,
    },
    /// The user asked to close the game.
    Quit,
    /// A window or display change; `detail` is the platform's description,
    /// recorded as a presentation diagnostic, and `exposed` asks for a redraw.
    Window {
        detail: String,
        exposed: bool,
    },
}
