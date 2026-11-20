//! A bindable input gesture and its configuration-file text form.

use super::keynames::{key_from_name, key_name};

/// SDL hat direction bits.
pub mod hat {
    pub const UP: u8 = 0x01;
    pub const RIGHT: u8 = 0x02;
    pub const DOWN: u8 = 0x04;
    pub const LEFT: u8 = 0x08;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    Key(i32),
    JoyAxis { port: u32, axis: i32, polarity: i32 },
    JoyButton { port: u32, button: i32 },
    JoyHat { port: u32, hat: i32, dir: u8 },
}

impl Gesture {
    /// Read `key NAME`, `joystick N axis M positive|negative`,
    /// `joystick N button M` or `joystick N hat M up|down|left|right`.
    pub fn parse(spec: &str) -> Option<Gesture> {
        let tokens: Vec<&str> = spec.split_whitespace().collect();
        let word = |index: usize, expected: &str| {
            tokens
                .get(index)
                .is_some_and(|token| token.eq_ignore_ascii_case(expected))
        };
        if word(0, "key") && tokens.len() >= 2 {
            return key_from_name(tokens[1]).map(Gesture::Key);
        }
        if !word(0, "joystick") || tokens.len() < 4 {
            return None;
        }
        let port = tokens[1].parse::<i32>().ok()? as u32;
        let index = tokens[3].parse::<i32>().ok()?;
        if word(2, "axis") && tokens.len() >= 5 {
            let polarity = if word(4, "positive") {
                1
            } else if word(4, "negative") {
                -1
            } else {
                return None;
            };
            Some(Gesture::JoyAxis {
                port,
                axis: index,
                polarity,
            })
        } else if word(2, "button") {
            Some(Gesture::JoyButton {
                port,
                button: index,
            })
        } else if word(2, "hat") && tokens.len() >= 5 {
            let dir = [
                ("up", hat::UP),
                ("down", hat::DOWN),
                ("left", hat::LEFT),
                ("right", hat::RIGHT),
            ]
            .into_iter()
            .find_map(|(name, bit)| word(4, name).then_some(bit))?;
            Some(Gesture::JoyHat {
                port,
                hat: index,
                dir,
            })
        } else {
            None
        }
    }
}

impl std::fmt::Display for Gesture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Gesture::Key(key) => write!(f, "key {}", key_name(key)),
            Gesture::JoyAxis {
                port,
                axis,
                polarity,
            } => write!(
                f,
                "joystick {port} axis {axis} {}",
                if polarity > 0 { "positive" } else { "negative" }
            ),
            Gesture::JoyButton { port, button } => write!(f, "joystick {port} button {button}"),
            Gesture::JoyHat { port, hat, dir } => {
                let dir = match dir {
                    hat::UP => "up",
                    hat::DOWN => "down",
                    hat::LEFT => "left",
                    hat::RIGHT => "right",
                    _ => "unknown",
                };
                write!(f, "joystick {port} hat {hat} {dir}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_documented_form_parses() {
        assert_eq!(Gesture::parse("key Down"), Some(Gesture::Key(0x4000_0051)));
        assert_eq!(Gesture::parse("KEY space"), Some(Gesture::Key(32)));
        assert_eq!(
            Gesture::parse("joystick 1 axis 2 negative"),
            Some(Gesture::JoyAxis {
                port: 1,
                axis: 2,
                polarity: -1
            })
        );
        assert_eq!(
            Gesture::parse("Joystick 0 BUTTON 7"),
            Some(Gesture::JoyButton { port: 0, button: 7 })
        );
        assert_eq!(
            Gesture::parse("joystick 0 hat 1 left"),
            Some(Gesture::JoyHat {
                port: 0,
                hat: 1,
                dir: hat::LEFT
            })
        );
    }

    #[test]
    fn malformed_specs_are_not_gestures() {
        for spec in [
            "",
            "banana pudding",
            "key",
            "key NoSuchKey",
            "joystick 0",
            "joystick x button 1",
            "joystick 0 button y",
            "joystick 0 axis 1",
            "joystick 0 axis 1 sideways",
            "joystick 0 hat 1",
            "joystick 0 hat 1 upish",
            "joystick 0 trigger 1",
        ] {
            assert_eq!(Gesture::parse(spec), None, "{spec:?}");
        }
    }

    #[test]
    fn display_writes_the_form_parse_reads() {
        for gesture in [
            Gesture::Key(32),
            Gesture::JoyAxis {
                port: 2,
                axis: 3,
                polarity: 1,
            },
            Gesture::JoyAxis {
                port: 2,
                axis: 3,
                polarity: -1,
            },
            Gesture::JoyButton { port: 0, button: 4 },
            Gesture::JoyHat {
                port: 1,
                hat: 0,
                dir: hat::DOWN,
            },
        ] {
            assert_eq!(Gesture::parse(&gesture.to_string()), Some(gesture));
        }
    }

    #[test]
    fn a_hat_with_an_odd_direction_displays_as_unknown() {
        let odd = Gesture::JoyHat {
            port: 0,
            hat: 0,
            dir: 3,
        };
        assert_eq!(odd.to_string(), "joystick 0 hat 0 unknown");
    }
}
