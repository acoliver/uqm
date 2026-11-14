//! Shared boundary for dialogue scripts while the surrounding engine still uses C.

pub(crate) use crate::comm::races::descriptor::AnimationDescriptor;
pub(crate) use crate::engine::StoryKey;

pub(crate) const ANIM_DISABLED: u8 = 1 << 7;
pub(crate) const ONE_SECOND: u16 = 840;
pub(crate) const FUEL_TANK_SCALE: u32 = 100;
pub(crate) const GLOBAL_PLAYER_NAME: i32 = -1_000_000;
pub(crate) const GLOBAL_SHIP_NAME: i32 = -999_999;

#[derive(Clone, Copy, Default)]
pub struct Supplies {
    pub landers: u8,
    pub fuel: u32,
    pub radioactives: u16,
}

pub trait DialogueHost<H> {
    fn get(&self, key: StoryKey) -> u8;
    fn set(&mut self, key: StoryKey, value: u8);
    fn npc(&mut self, phrase: i32);
    fn response(&mut self, phrase: u16, handler: H, fragments: &[i32]);
    fn enabled(&self, phrase: u16) -> bool;
    fn disable(&mut self, phrase: u16);
    fn alliance_fragments(&self, first: u16) -> Vec<i32> {
        let name = i32::from(self.get(StoryKey::NEW_ALLIANCE_NAME));
        let first = i32::from(first);
        let mut fragments = vec![first + name];
        if name == 3 {
            fragments.extend([GLOBAL_PLAYER_NAME, first + 4]);
        }
        fragments
    }
    fn supplies(&self) -> Supplies;
    fn add_lander(&mut self);
    fn gauges(&mut self, crew: i16, fuel: i16, units: i32);
    fn palette(&mut self, index: u16);
    fn talk_segue(&mut self, track: u16);
    fn disable_animation(&mut self, index: usize);
    fn fade_palette(&mut self, index: u16, duration: i16);
}

pub(crate) fn animation(
    start_index: u16,
    num_frames: u8,
    anim_flags: u8,
    frame_rate: (u16, u16),
    restart_rate: (u16, u16),
) -> AnimationDescriptor {
    AnimationDescriptor {
        start_index,
        num_frames,
        anim_flags,
        base_frame_rate: frame_rate.0,
        random_frame_rate: frame_rate.1,
        base_restart_rate: restart_rate.0,
        random_restart_rate: restart_rate.1,
        block_mask: 0,
    }
}
