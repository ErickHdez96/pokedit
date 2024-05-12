#[cfg(feature = "simulator")]
use sdl2::keyboard::Keycode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
    Autorepeat(Key),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    X,
    Y,
    Start,
    Select,
    L,
    R,
    Menu,
    L2,
    R2,
    Power,
    VolDown,
    VolUp,
    Quit,
    Unknown,
}

#[cfg(feature = "simulator")]
impl From<Keycode> for Key {
    fn from(value: Keycode) -> Self {
        match value {
            Keycode::Up => Key::Up,
            Keycode::Down => Key::Down,
            Keycode::Left => Key::Left,
            Keycode::Right => Key::Right,
            Keycode::Space => Key::A,
            Keycode::LCtrl => Key::B,
            Keycode::LShift => Key::X,
            Keycode::LAlt => Key::Y,
            Keycode::Return => Key::Start,
            Keycode::RCtrl => Key::Select,
            Keycode::E => Key::L,
            Keycode::T => Key::R,
            Keycode::Escape => Key::Menu,
            Keycode::Tab => Key::L2,
            Keycode::Backspace => Key::R2,
            Keycode::Power => Key::Power,
            Keycode::LGui => Key::VolDown,
            Keycode::RGui => Key::VolUp,
            _ => Key::Unknown,
        }
    }
}
