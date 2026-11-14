#[derive(Clone, Copy, Default)]
pub(super) struct CreditsKeys {
    pub select: bool,
    pub cancel: bool,
    pub up: bool,
    pub down: bool,
}

pub(super) struct CreditsControl {
    pub rate: i32,
    pub running: bool,
    pub outtakes: bool,
    pub victory: bool,
    allow_cancel: bool,
    close_when_done: bool,
    pub close_time_out: u32,
}

impl CreditsControl {
    pub(super) fn new(victory: bool) -> Self {
        Self {
            rate: 9,
            running: true,
            outtakes: victory,
            victory,
            allow_cancel: !victory,
            close_when_done: !victory,
            close_time_out: 0,
        }
    }

    pub(super) fn input(&mut self, keys: CreditsKeys, abort: bool, now: u32) -> bool {
        if self.running {
            self.close_time_out = 0;
        }
        if abort || (self.allow_cancel && (keys.select || keys.cancel)) {
            return false;
        }
        if keys.up || keys.down {
            let step = self.rate.abs() / 5 + 1;
            self.rate = (self.rate + if keys.down { step } else { -step }).clamp(-130, 130);
        }
        if !self.running {
            self.allow_cancel = true;
            if self.close_when_done {
                if self.close_time_out == 0 {
                    self.close_time_out = now.wrapping_add(840 * 5);
                } else if now > self.close_time_out {
                    return false;
                }
            }
            if keys.select || keys.cancel {
                return false;
            }
        }
        true
    }
}
