use super::*;
use crate::math::RandomContext;

fn position(s: &CSisState) -> (i16, i16) {
    // units.h, original hyperspace speed factors (not normalized).
    (
        ((s.log_x.wrapping_mul(6250).wrapping_add(49920)) / 99840) as i16,
        (9999 - (s.log_y.wrapping_mul(625).wrapping_add(6000)) / 12000) as i16,
    )
}
impl<A: MelnormApi> Dialogue<'_, A> {
    pub(super) fn ship_worth(&self) -> u16 {
        let s = self.api.ship();
        let mut worth = u16::from(s.num_landers) * u16::from(self.api.module_cost(0));
        for m in s.drive_slots {
            if m < 20 {
                worth = worth.wrapping_add(u16::from(self.api.module_cost(1)));
            }
        }
        for m in s.jet_slots {
            if m < 20 {
                worth = worth.wrapping_add(u16::from(self.api.module_cost(2)));
            }
        }
        let mut pods = -(s.crew_enlisted.div_ceil(50) as i8);
        for m in s.module_slots {
            if m < 14 {
                let removable = if m == 3 {
                    pods = pods.wrapping_add(1);
                    pods > 0
                } else {
                    true
                };
                if removable {
                    worth = worth.wrapping_add(u16::from(self.api.module_cost(usize::from(m))));
                }
            }
        }
        worth
    }
    fn strip_seed(&self) -> u32 {
        let s = self.api.ship();
        let (x, y) = position(&s);
        (i32::from(y) / 100) as u32 * 1013
            + (i32::from(x) / 100) as u32
            + u32::from(s.num_landers)
            + [1, 4, 7, 10]
                .into_iter()
                .map(|i| u32::from(s.module_slots[i]))
                .sum::<u32>()
    }
    pub(super) fn strip_ship(&mut self, mut fuel: u16) -> bool {
        self.api.set("MELNORME_RESCUE_REFUSED", 0);
        let mut ship = self.api.ship();
        let mut pods = -(ship.crew_enlisted.div_ceil(50) as i8);
        if fuel == 0 {
            self.api.set_ship(self.session.sis_copy);
            self.api
                .gauges(0x7fff, self.session.rescue_fuel as i16, 0x7fff);
        } else if fuel == u16::MAX {
            ship.num_landers = 0;
            ship.drive_slots = [20; 11];
            ship.jet_slots = [21; 8];
            ship.fuel_on_board = ship.fuel_on_board.min(1000);
            ship.total_bio_mass = 0;
            ship.total_element_mass = 0;
            ship.element_amounts = [0; 8];
            for m in &mut ship.module_slots {
                if *m < 14 {
                    let removable = if *m == 3 {
                        pods = pods.wrapping_add(1);
                        pods > 0
                    } else {
                        true
                    };
                    if removable {
                        *m = 22;
                    }
                }
            }
            self.api.set_ship(ship);
            self.api.gauges(0x7fff, 0x7fff, 0x7fff);
        } else {
            let mut rng = RandomContext::with_seed(self.strip_seed());
            self.session.sis_copy = ship;
            let mut counts = [0u8; 14];
            let capacity = self.capacity();
            if u32::from(fuel) > capacity {
                fuel = capacity as u16;
            }
            let mut bays = -(ship.total_element_mass.div_ceil(500) as i8);
            for m in ship.module_slots {
                if m == 3 {
                    pods = pods.wrapping_add(1);
                } else if m == 4 {
                    bays = bays.wrapping_add(1);
                }
            }
            let worth = fuel / 100;
            let mut total = 0u16;
            let mut searches = 0u8;
            let mut first = 255u8;
            let mut last = 255u8;
            while total < worth && self.ship_worth() != 0 {
                searches = searches.wrapping_add(1);
                if searches == 0 {
                    break;
                }
                let random = rng.next_random();
                let mut module = (random as u8) % 4;
                let copy = &mut self.session.sis_copy;
                match module {
                    0 => {
                        if copy.num_landers == 0 {
                            continue;
                        }
                        copy.num_landers -= 1;
                    }
                    1 => {
                        let Some(i) = copy.drive_slots.iter().position(|&m| m < 20) else {
                            continue;
                        };
                        copy.drive_slots[i] = 20;
                    }
                    2 => {
                        let Some(i) = copy.jet_slots.iter().position(|&m| m < 20) else {
                            continue;
                        };
                        copy.jet_slots[i] = 21;
                    }
                    3 => {
                        let i = usize::from(((random >> 8) as u8) % 16);
                        module = copy.module_slots[i];
                        if module >= 14
                            || module == 5
                            || module == 6
                            || (module == 4 && i16::from(counts[4]) >= i16::from(bays))
                            || (module == 3 && i16::from(counts[3]) >= i16::from(pods))
                        {
                            continue;
                        }
                        copy.module_slots[i] = 22;
                    }
                    _ => unreachable!(),
                }
                if first == 255 {
                    first = module;
                    last = module;
                } else if module > last {
                    last = module;
                }
                counts[usize::from(module)] += 1;
                total = total.wrapping_add(u16::from(self.api.module_cost(usize::from(module))));
            }
            if total == 0 {
                self.api.npc(i32::from(CHARITY));
                self.api.gauges(0, fuel as i16, 0);
                return false;
            }
            self.api.npc(i32::from(RESCUE_OFFER));
            self.session.rescue_fuel = fuel;
            self.api.npc(i32::from(if u32::from(fuel) == capacity {
                RESCUE_TANKS
            } else {
                RESCUE_HOME
            }));
            for (i, &count) in counts.iter().enumerate() {
                if count != 0 {
                    if i == usize::from(last) && i != usize::from(first) {
                        self.api.npc(i32::from(END_LIST_WITH_AND));
                    }
                    self.api
                        .npc(i32::from(ENUMERATE_ONE + u16::from(count - 1)));
                    self.api.npc(i32::from(match i {
                        0 => LANDERS,
                        1 => THRUSTERS,
                        2 => JETS,
                        3 => PODS,
                        4 => BAYS,
                        7 => DYNAMOS,
                        8 => FURNACES,
                        9 => GUNS,
                        10 => BLASTERS,
                        11 => CANNONS,
                        12 => TRACKERS,
                        13 => DEFENSES,
                        _ => panic!("unknown stripped module"),
                    }));
                }
            }
        }
        true
    }
    pub(super) fn do_rescue(&mut self, _r: u16) {
        let (x, y) = position(&self.api.ship());
        let dx = x.wrapping_sub(1752);
        let dy = y.wrapping_sub(1450);
        let distance = (i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as u32;
        let fuel = crate::math::square_root(distance).wrapping_add(200);
        if self.strip_ship(fuel) {
            self.api.response(TAKE_IT, Handler::ExitConversation, &[]);
            self.api.response(LEAVE_IT, Handler::ExitConversation, &[]);
        }
    }
}
