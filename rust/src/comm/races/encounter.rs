//! Encounter effects shared by the Starbase, Spathi and Ilwrath scripts.

pub(crate) use super::script::DialogueHost;

pub(crate) trait EncounterApi<H>: DialogueHost<H> {
    fn segue(&mut self, segue: u32);
    fn get_segue(&self) -> u32;
    fn activity(&self) -> u16;
    fn alliance(&self, race: u16) -> u16;
    fn escort_feasibility(&self, race: u16) -> u16;
    fn add_escorts(&mut self, race: u16, count: i16);
    fn fwiffo_crew(&mut self);
    fn advance_ilwrath(&mut self);
}

#[cfg(test)]
pub(crate) mod testing;
