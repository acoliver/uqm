use super::*;

impl<A: SlylandApi> Dialogue<'_, A> {
    pub(super) fn initialize(&mut self) {
        self.api.segue(1);
    }

    pub(super) fn handle(&mut self, handler: Handler, response: i32) {
        match handler {
            Handler::Combat => self.combat(response),
        }
    }

    pub(super) fn say_coord(&mut self, coordinate: i16) {
        let coordinate = i32::from(coordinate);
        let magnitude = coordinate.abs();
        self.api.splice(if coordinate < 0 {
            COORD_MINUS
        } else {
            COORD_PLUS
        });
        self.api.number(magnitude / 10, true);
        self.api.splice(COORD_POINT);
        self.api.number(magnitude % 10, false);
    }

    pub(super) fn combat(&mut self, response: i32) {
        let topics = [
            (
                "SLYLANDRO_PROBE_THREAT",
                [THREAT_1, THREAT_2, THREAT_3, THREAT_4],
                [
                    PROGRAMMED_TO_DEFEND_1,
                    PROGRAMMED_TO_DEFEND_2,
                    PROGRAMMED_TO_DEFEND_3,
                    PROGRAMMED_TO_DEFEND_4,
                ],
            ),
            (
                "SLYLANDRO_PROBE_WRONG",
                [
                    SOMETHING_WRONG_1,
                    SOMETHING_WRONG_2,
                    SOMETHING_WRONG_3,
                    SOMETHING_WRONG_4,
                ],
                [
                    NOMINAL_FUNCTION_1,
                    NOMINAL_FUNCTION_2,
                    NOMINAL_FUNCTION_3,
                    NOMINAL_FUNCTION_4,
                ],
            ),
            (
                "SLYLANDRO_PROBE_ID",
                [WE_ARE_US_1, WE_ARE_US_2, WE_ARE_US_3, WE_ARE_US_4],
                [
                    THIS_IS_PROBE_1,
                    THIS_IS_PROBE_2,
                    THIS_IS_PROBE_3,
                    THIS_IS_PROBE_40,
                ],
            ),
            (
                "SLYLANDRO_PROBE_INFO",
                [WHY_ATTACK_1, WHY_ATTACK_2, WHY_ATTACK_3, WHY_ATTACK_4],
                [
                    PEACEFUL_MISSION_1,
                    PEACEFUL_MISSION_2,
                    PEACEFUL_MISSION_3,
                    PEACEFUL_MISSION_4,
                ],
            ),
            (
                "SLYLANDRO_PROBE_EXIT",
                [BYE_1, BYE_2, BYE_3, BYE_4],
                [GOODBYE_1, GOODBYE_2, GOODBYE_3, GOODBYE_4],
            ),
        ];
        if response == 0 {
            if self.api.get("DESTRUCT_CODE_ON_SHIP") != 0 {
                self.api
                    .response(DESTRUCT_CODE as u16, Handler::Combat, &[]);
            }
            let mut choices = [0; 5];
            for (i, (key, phrases, _)) in topics.iter().enumerate() {
                let n = self.api.get(key);
                choices[i] = phrases[usize::from(n.min(3))] as u16;
                self.api.response(choices[i], Handler::Combat, &[]);
            }
            self.api.remember_choices(choices);
            return;
        }
        if response == DESTRUCT_CODE as i32 {
            self.api.npc(DESTRUCT_SEQUENCE as i32);
            self.api.segue(2);
            return;
        }
        if let Some(i) = self
            .api
            .choices()
            .iter()
            .position(|&p| i32::from(p) == response)
        {
            let (key, _, phrases) = topics[i];
            let n = self.api.get(key);
            if let Some(&phrase) = phrases.get(usize::from(n)) {
                self.api.npc(phrase as i32);
                if i == 2 && n == 3 {
                    let (dx, dy) = coordinates(self.api.position());
                    self.say_coord(dy);
                    self.api.splice(THIS_IS_PROBE_41);
                    self.say_coord(dx);
                    self.api.npc(THIS_IS_PROBE_42 as i32);
                }
            }
            self.api
                .set(key, if n == 3 { n } else { n.wrapping_add(1) });
        }
        self.api.npc(HOSTILE as i32);
        self.api.set("PROBE_EXHIBITED_BUG", 1);
        self.api.segue(1);
    }

    pub(super) fn intro(&mut self) {
        let n = self.api.get("SLYLANDRO_PROBE_VISITS");
        if let Some(&p) = [
            WE_COME_IN_PEACE_1,
            WE_COME_IN_PEACE_2,
            WE_COME_IN_PEACE_3,
            WE_COME_IN_PEACE_4,
            WE_COME_IN_PEACE_5,
            WE_COME_IN_PEACE_6,
            WE_COME_IN_PEACE_7,
            WE_COME_IN_PEACE_8,
        ]
        .get(usize::from(n))
        {
            self.api.npc(p as i32);
        }
        self.api.set(
            "SLYLANDRO_PROBE_VISITS",
            if n == 7 { n } else { n.wrapping_add(1) },
        );
        self.combat(0);
    }
}

fn coordinates((log_x, log_y, width, height): (i32, i32, i32, i32)) -> (i16, i16) {
    // units.h converts each universe coordinate to COORD before the offsets
    // are applied and the results are stored in SIZE.
    let units_x = (((width - 64) << 5) >> 4) * 195;
    let units_y = ((height << 5) >> 4) * 25;
    let x = (log_x.wrapping_mul(6250).wrapping_add(units_x >> 1) / units_x) as i16;
    let y = (9999 - log_y.wrapping_mul(625).wrapping_add(units_y >> 1) / units_y) as i16;
    ((i32::from(x) - 333) as i16, (9812 - i32::from(y)) as i16)
}
