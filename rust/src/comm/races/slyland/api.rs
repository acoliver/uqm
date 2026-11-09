use super::Handler;
use crate::comm::races::encounter::EncounterApi;

pub(crate) trait SlylandApi: EncounterApi<Handler> {
    fn choices(&self) -> [u16; 5];
    fn remember_choices(&mut self, choices: [u16; 5]);
    fn position(&self) -> (i32, i32, i32, i32);
    fn splice(&mut self, phrase: u32);
    fn number(&mut self, number: i32, padded: bool);
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Speech {
    Splice(u32),
    Number(i32, bool),
}
#[cfg(test)]
pub(crate) struct World {
    pub choices: [u16; 5],
    pub position: (i32, i32, i32, i32),
    pub speech: Vec<(usize, Speech)>,
}
#[cfg(test)]
impl Default for World {
    fn default() -> Self {
        Self {
            choices: [0; 5],
            position: (0, 0, 320, 240),
            speech: Vec::new(),
        }
    }
}
#[cfg(test)]
impl SlylandApi for crate::comm::races::encounter::testing::Fake<Handler, World> {
    fn choices(&self) -> [u16; 5] {
        self.world.choices
    }
    fn remember_choices(&mut self, choices: [u16; 5]) {
        self.world.choices = choices;
    }
    fn position(&self) -> (i32, i32, i32, i32) {
        self.world.position
    }
    fn splice(&mut self, phrase: u32) {
        self.world
            .speech
            .push((self.effects.len(), Speech::Splice(phrase)));
    }
    fn number(&mut self, number: i32, padded: bool) {
        self.world
            .speech
            .push((self.effects.len(), Speech::Number(number, padded)));
    }
}
