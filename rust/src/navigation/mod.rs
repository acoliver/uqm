//! Navigation-world state, encounters and movement.

#[allow(
    clippy::all,
    dead_code,
    improper_ctypes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    reason = "Rust-owned engine ABI declarations, including opaque resources"
)]
pub mod abi {
    include!("../abi/navigation_abi.rs");
}

pub mod events;
#[cfg(not(test))]
pub mod globals;
pub mod groups;
pub mod hyper_data;
pub mod hyper_encounters;
pub mod hyper_menu;
pub mod hyper_radar;
pub mod hyper_transition;
pub mod hyper_units;
pub mod hyperspace;
pub mod initial_state;
pub mod interplanetary;

#[cfg(not(test))]
pub(crate) fn get(name: &str) -> u8 {
    let (start, end) = crate::state::game_state::lookup_game_state_bits(name)
        .expect("navigation state key exists");
    unsafe { crate::state::ffi::rust_get_game_state_bits(start as i32, end as i32) }
}

#[cfg(not(test))]
pub(crate) fn set(name: &str, value: u8) {
    let (start, end) = crate::state::game_state::lookup_game_state_bits(name)
        .expect("navigation state key exists");
    unsafe { crate::state::ffi::rust_set_game_state_bits(start as i32, end as i32, value) }
}

fn color(r: u8, g: u8, b: u8) -> crate::graphics::native_abi::Color {
    crate::graphics::native_abi::Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}

#[cfg(test)]
mod color_tests {
    #[test]
    fn five_bit_channels_expand_without_changing_hyperspace_colors() {
        for value in 0..32 {
            let color = super::color(value, value, value);
            let channel = (value << 3) | (value >> 2);
            assert_eq!(
                (color.r, color.g, color.b, color.a),
                (channel, channel, channel, 255)
            );
        }
        let color = super::color(0, 14, 31);
        assert_eq!((color.r, color.g, color.b, color.a), (0, 115, 255, 255));
    }
}
