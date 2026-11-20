//! Crew and energy record changes before the supplied presentation effect.
use super::flags::{IN_LAST_BATTLE, LOW_ON_ENERGY, NPC_PLAYER_NUM};
use super::{Cx, ElementId};

fn energy_delta(current: u8, maximum: u8, delta: i16) -> (bool, i16) {
    if delta >= 0 {
        if current.wrapping_add(delta as u8) > maximum {
            (true, i16::from(maximum) - i16::from(current))
        } else {
            (true, delta)
        }
    } else {
        ((-i32::from(delta)) as u8 <= current, delta)
    }
}
fn crew_delta(current: u16, original: u16, maximum: u16, delta: i16) -> (bool, u16, i16) {
    if delta > 0 {
        let updated = current.wrapping_add(delta as u16);
        if updated > maximum {
            (true, maximum, maximum.wrapping_sub(original) as i16)
        } else {
            (true, updated, delta)
        }
    } else if delta < 0 {
        if current > (-i32::from(delta)) as u16 {
            (true, current.wrapping_add(delta as u16), delta)
        } else {
            (false, 0, (-i32::from(current as i16)) as i16)
        }
    } else {
        (true, current, 0)
    }
}
pub fn delta_energy(cx: &mut Cx<'_>, e: ElementId, delta: i16) -> bool {
    let ship = cx.owner(e);
    let info = cx.race_stats(ship).ship_info;
    let (success, adjusted) = energy_delta(info.energy_level, info.max_energy, delta);
    if !success {
        cx.ship_state_mut(ship).status |= LOW_ON_ENERGY as u16;
    } else {
        cx.ship_state_mut(ship).status &= !(LOW_ON_ENERGY as u16);
        cx.ship_state_mut(ship).energy_counter = cx.race_stats(ship).characteristics.energy_wait;
        cx.draw_statistics(ship, 0, adjusted);
    }
    success
}
pub fn delta_crew(cx: &mut Cx<'_>, e: ElementId, delta: i16) -> bool {
    if cx.world.activity() as u8 == IN_LAST_BATTLE as u8
        && cx.element(e).player == NPC_PLAYER_NUM as i16
    {
        return true;
    }
    let ship = cx.owner(e);
    let info = cx.race_stats(ship).ship_info;
    let (success, updated, adjusted) =
        crew_delta(cx.element(e).hits, info.crew_level, info.max_crew, delta);
    cx.element_mut(e).hits = updated;
    cx.draw_statistics(ship, adjusted, 0);
    success
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn energy_checks_narrow_but_do_not_replace_the_accepted_delta() {
        assert_eq!(energy_delta(250, 255, 10), (true, 10));
        assert_eq!(energy_delta(200, 255, 256), (true, 256));
        assert_eq!(energy_delta(12, 20, 10), (true, 8));
        assert_eq!(energy_delta(2, 255, -3), (false, -3));
        assert_eq!(energy_delta(12, 20, -12), (true, -12));
        assert_eq!(energy_delta(0, 255, i16::MIN), (true, i16::MIN));
    }
    #[test]
    fn crew_clamp_uses_ship_info_before_update() {
        assert_eq!(crew_delta(9, 7, 10, 4), (true, 10, 3));
        assert_eq!(crew_delta(3, 3, 10, -3), (false, 0, -3));
        assert_eq!(crew_delta(4, 4, 10, -3), (true, 1, -3));
        assert_eq!(crew_delta(65535, 65535, 65535, 1), (true, 0, 1));
        assert_eq!(
            crew_delta(32768, 32768, 65535, i16::MIN),
            (false, 0, i16::MIN)
        );
    }
}
