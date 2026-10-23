//! The module chooser: `DoInstallModule` (`outfit.c:171-535`) as a pure
//! frame step.
//!
//! The player picks a slot (a drive, a jet, a module slot or a lander
//! berth), and either removes what is in it or buys something for it. The C
//! code keeps all of that in `MENU_STATE.CurState`, which holds a piece
//! number while the player chooses what to buy and `EMPTY_SLOT + n` while
//! the player chooses a slot. [`Cursor`] names those two cases.
//!
//! The purchase and removal rules are the ones in [`super::module_guard`].
//! This module decides what a frame means and what the player sees next. The
//! screen in [`super::module_screen`] carries the answer out.

use super::gfx::{Point, Rect};
use super::ids::{
    CHMMR_BOMB_INSTALLED, DRIVE_FLASH_SIZE, DRIVE_TOP, JET_FLASH_SIZE, JET_TOP, LANDER_FLASH_SIZE,
    LANDER_WIDTH, LANDER_X, LANDER_Y, MAX_LANDERS, MODULE_FLASH_SIZE, MODULE_TOP, NUM_BOMB_MODULES,
    NUM_DRIVE_SLOTS, NUM_JET_SLOTS, SHIP_PIECE_OFFSET, WEAPON_EXCLUDED_SLOTS,
};
use super::input::MenuPulses;
use super::module_guard::{
    check_module_purchase, check_module_removal, module_price, ModuleBlock, ModuleCheck,
    ModuleCosts, ShipLoad, SlotIndexOutOfRange,
};
use super::platform::SisView;
use super::sis_modules::{
    ModuleSlot, ModuleSlots, ModuleSlotsError, ModuleType, EMPTY_SLOT, NUM_MODULE_SLOTS,
};
use crate::mainloop::types::{activity_flags, ActivityValue};

/// The first module the player may buy for a module slot (`CREW_POD`).
const FIRST_PURCHASE_MODULE: ModuleType = ModuleType::CrewPod;
/// The last module the player may buy (`NUM_PURCHASE_MODULES - 1`). The
/// bomb modules after it come with the Chmmr bomb.
const LAST_PURCHASE_MODULE: ModuleType = ModuleType::AntimissileDefense;

/// Which kind of slot the player is choosing among. The kinds are the four
/// `EMPTY_SLOT + n` values of `MENU_STATE.CurState`, in that order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotKind {
    /// `EMPTY_SLOT + 0`: a thruster slot.
    Drive,
    /// `EMPTY_SLOT + 1`: a turning jet slot.
    Jet,
    /// `EMPTY_SLOT + 2`: a module slot.
    Module,
    /// `EMPTY_SLOT + 3`: a lander berth.
    Lander,
}

impl SlotKind {
    /// The byte an empty slot of this kind holds, which is also the
    /// `CurState` value while the player chooses among these slots.
    #[must_use]
    pub const fn empty_piece(self) -> u8 {
        EMPTY_SLOT
            + match self {
                SlotKind::Drive => 0,
                SlotKind::Jet => 1,
                SlotKind::Module => 2,
                SlotKind::Lander => 3,
            }
    }

    /// How many slots of this kind the ship shows.
    #[must_use]
    pub const fn slot_count(self) -> u8 {
        match self {
            SlotKind::Drive => NUM_DRIVE_SLOTS as u8,
            SlotKind::Jet => NUM_JET_SLOTS as u8,
            SlotKind::Module => NUM_MODULE_SLOTS as u8,
            SlotKind::Lander => MAX_LANDERS,
        }
    }

    /// The kind of slot a purchasable piece goes into: the `switch` on
    /// `CurState` in `DoInstallModule`.
    #[must_use]
    pub const fn holding(piece: ModuleType) -> Self {
        match piece {
            ModuleType::PlanetLander => SlotKind::Lander,
            ModuleType::FusionThruster => SlotKind::Drive,
            ModuleType::TurningJets => SlotKind::Jet,
            _ => SlotKind::Module,
        }
    }

    /// What the player is offered first when choosing an empty slot of this
    /// kind (`new_slot_piece -= EMPTY_SLOT - 1`, `outfit.c:305-309`).
    #[must_use]
    pub const fn first_offer(self) -> ModuleType {
        match self {
            SlotKind::Drive => ModuleType::FusionThruster,
            SlotKind::Jet => ModuleType::TurningJets,
            SlotKind::Module => FIRST_PURCHASE_MODULE,
            SlotKind::Lander => ModuleType::PlanetLander,
        }
    }

    const fn next(self) -> Self {
        match self {
            SlotKind::Drive => SlotKind::Jet,
            SlotKind::Jet => SlotKind::Module,
            SlotKind::Module => SlotKind::Lander,
            SlotKind::Lander => SlotKind::Drive,
        }
    }

    const fn previous(self) -> Self {
        match self {
            SlotKind::Drive => SlotKind::Lander,
            SlotKind::Jet => SlotKind::Drive,
            SlotKind::Module => SlotKind::Jet,
            SlotKind::Lander => SlotKind::Module,
        }
    }

    /// The first and last slot the cursor visits (`FirstItem`, `LastItem`).
    /// With the Chmmr bomb installed the bomb holds the first module slots.
    const fn item_range(self, bomb_installed: bool) -> (u8, u8) {
        let first = match self {
            SlotKind::Module if bomb_installed => NUM_BOMB_MODULES,
            _ => 0,
        };
        (first, self.slot_count() - 1)
    }
}

/// What `CurState` stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    /// The player is choosing a slot.
    Slots(SlotKind),
    /// The player is choosing what to buy for an empty slot.
    Offer(ModuleType),
}

impl Cursor {
    fn kind(self) -> SlotKind {
        match self {
            Cursor::Slots(kind) => kind,
            Cursor::Offer(piece) => SlotKind::holding(piece),
        }
    }
}

/// The drives, jets, modules and landers the flagship carries, checked
/// against the bytes the C code stores (`globdata.c:386-401`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fittings {
    landers: u8,
    drives: [bool; NUM_DRIVE_SLOTS],
    jets: [bool; NUM_JET_SLOTS],
    modules: ModuleSlots,
}

/// Slot data the chooser cannot make sense of, and chooser states that
/// cannot happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ModuleChooserError {
    #[error(
        "drive slot {index} holds byte {value}, neither FUSION_THRUSTER nor an empty drive slot"
    )]
    DriveSlot { index: usize, value: u8 },
    #[error("jet slot {index} holds byte {value}, neither TURNING_JETS nor an empty jet slot")]
    JetSlot { index: usize, value: u8 },
    #[error("{count} landers exceed the {MAX_LANDERS} berths")]
    TooManyLanders { count: u8 },
    #[error("ship module slots: {0}")]
    ModuleSlots(#[from] ModuleSlotsError),
    #[error("module removal check: {0}")]
    SlotIndex(#[from] SlotIndexOutOfRange),
    #[error("no module in {first:?}..={last:?} can be offered for slot {slot}")]
    NothingToOffer {
        first: ModuleType,
        last: ModuleType,
        slot: u8,
    },
    #[error("the {offered:?} offer is open on an occupied {kind:?} slot {slot}")]
    OfferedSlotOccupied {
        offered: ModuleType,
        kind: SlotKind,
        slot: u8,
    },
}

impl Fittings {
    pub fn from_view(view: &SisView) -> Result<Self, ModuleChooserError> {
        if view.num_landers > MAX_LANDERS {
            return Err(ModuleChooserError::TooManyLanders {
                count: view.num_landers,
            });
        }
        let mut drives = [false; NUM_DRIVE_SLOTS];
        for (index, (&value, fitted)) in view.drive_slots.iter().zip(&mut drives).enumerate() {
            *fitted = match ModuleType::from_u8(value) {
                Some(ModuleType::FusionThruster) => true,
                None if value == SlotKind::Drive.empty_piece() => false,
                _ => return Err(ModuleChooserError::DriveSlot { index, value }),
            };
        }
        let mut jets = [false; NUM_JET_SLOTS];
        for (index, (&value, fitted)) in view.jet_slots.iter().zip(&mut jets).enumerate() {
            *fitted = match ModuleType::from_u8(value) {
                Some(ModuleType::TurningJets) => true,
                None if value == SlotKind::Jet.empty_piece() => false,
                _ => return Err(ModuleChooserError::JetSlot { index, value }),
            };
        }
        Ok(Self {
            landers: view.num_landers,
            drives,
            jets,
            modules: ModuleSlots::from_raw(&view.module_slots)?,
        })
    }

    /// What `old_slot_piece` holds for slot `item` of `kind`; `None` for an
    /// empty slot. A lander berth is filled while it is below the lander
    /// count.
    fn piece_at(&self, kind: SlotKind, item: u8) -> Result<Option<ModuleType>, ModuleChooserError> {
        let index = usize::from(item);
        Ok(match kind {
            SlotKind::Lander => (item < self.landers).then_some(ModuleType::PlanetLander),
            SlotKind::Drive => self.drives[index].then_some(ModuleType::FusionThruster),
            SlotKind::Jet => self.jets[index].then_some(ModuleType::TurningJets),
            SlotKind::Module => match self
                .modules
                .get(index)
                .ok_or(SlotIndexOutOfRange { slot_index: index })?
            {
                ModuleSlot::Empty => None,
                ModuleSlot::Installed(module) => Some(module),
            },
        })
    }

    /// Where a purchase or removal for `item` of `kind` lands.
    fn placement(&self, kind: SlotKind, item: u8, installing: bool) -> Placement {
        match kind {
            SlotKind::Drive => Placement::Drive(item),
            SlotKind::Jet => Placement::Jet(item),
            SlotKind::Module => Placement::Module(item),
            // An empty berth is at or past the lander count and below
            // MAX_LANDERS, so one more lander fits. A filled berth is below
            // the count, so there is one to remove.
            SlotKind::Lander => Placement::Landers {
                count_after: if installing {
                    self.landers + 1
                } else {
                    self.landers - 1
                },
            },
        }
    }
}

/// Inputs of one frame.
#[derive(Debug, Clone, Copy)]
pub struct ChooserInput<'a> {
    pub pulses: MenuPulses,
    /// `GLOBAL (CurrentActivity)`.
    pub activity: ActivityValue,
    pub ship: &'a SisView,
    /// `GET_GAME_STATE (CHMMR_BOMB_STATE)`.
    pub chmmr_bomb_state: u32,
}

/// Where a change goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Drive(u8),
    Jet(u8),
    Module(u8),
    /// Landers have no per-berth state: the new lander count is what is
    /// stored (`++NumLanders`, `--NumLanders`).
    Landers {
        count_after: u8,
    },
}

impl Placement {
    #[must_use]
    pub const fn kind(self) -> SlotKind {
        match self {
            Placement::Drive(_) => SlotKind::Drive,
            Placement::Jet(_) => SlotKind::Jet,
            Placement::Module(_) => SlotKind::Module,
            Placement::Landers { .. } => SlotKind::Lander,
        }
    }
}

/// What the player's purchase or removal does to RU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Build `module`; its price is paid.
    Install { module: ModuleType, price: u32 },
    /// Take `module` out; its price is refunded.
    Remove { module: ModuleType, refund: u32 },
}

/// A purchase or removal to carry out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transaction {
    pub placement: Placement,
    pub change: Change,
}

impl Transaction {
    /// The slot byte after the change (`new_slot_piece`).
    #[must_use]
    pub const fn new_piece(&self) -> u8 {
        match self.change {
            Change::Install { module, .. } => module as u8,
            Change::Remove { .. } => self.placement.kind().empty_piece(),
        }
    }

    /// The slot byte before the change (`old_slot_piece`).
    #[must_use]
    pub const fn old_piece(&self) -> u8 {
        match self.change {
            Change::Install { .. } => self.placement.kind().empty_piece(),
            Change::Remove { module, .. } => module as u8,
        }
    }

    /// The piece the strip shows afterwards.
    #[must_use]
    pub const fn shown(&self) -> Option<ModuleType> {
        match self.change {
            Change::Install { module, .. } => Some(module),
            Change::Remove { .. } => None,
        }
    }

    /// RU added to the balance: a refund is positive, a price negative.
    #[must_use]
    pub fn res_units_delta(&self) -> i32 {
        match self.change {
            Change::Install { price, .. } => {
                -i32::try_from(price).expect("a module price is at most 255 * 50 RU")
            }
            Change::Remove { refund, .. } => {
                i32::try_from(refund).expect("a module price is at most 255 * 50 RU")
            }
        }
    }
}

/// What a frame decided. `shown` is `None` when the strip should be blank:
/// `DrawModuleStrings` was given an `EMPTY_SLOT` byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChooserStep {
    /// Nothing changed.
    Idle,
    /// `CHECK_ABORT`: give control back to the Outfit screen, which leaves.
    Abort,
    /// First frame: clear the radar, show what the first slot holds and
    /// flash the slot.
    Open { shown: Option<ModuleType> },
    /// The change is not allowed: `MENU_SOUND_FAILURE`.
    Refused(ModuleBlock),
    /// An empty slot was chosen: start offering `piece` for it.
    Offer { piece: ModuleType },
    /// The offer moved to another module.
    ShowOffer { piece: ModuleType },
    /// Cancel while offering: back to choosing among the slots.
    BackToSlots,
    /// The slot cursor moved.
    Move { shown: Option<ModuleType> },
    /// Carry out a purchase or removal.
    Apply(Transaction),
    /// Cancel while choosing a slot: back to the Outfit menu.
    Leave,
}

/// `MENU_STATE.CurState`, `delta_item` and `Initialized` for the chooser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleChooser {
    cursor: Cursor,
    item: u8,
    opened: bool,
}

impl ModuleChooser {
    /// The chooser as `DoOutfit` sets it up for the Modules entry
    /// (`outfit.c:737-745`).
    #[must_use]
    pub const fn new(chmmr_bomb_state: u32) -> Self {
        let kind = SlotKind::Module;
        Self {
            cursor: Cursor::Slots(kind),
            item: kind.item_range(chmmr_bomb_state == CHMMR_BOMB_INSTALLED).0,
            opened: false,
        }
    }

    #[must_use]
    pub const fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// `delta_item`: the slot of the current kind the cursor is on.
    #[must_use]
    pub const fn item(&self) -> u8 {
        self.item
    }

    /// One `DoInstallModule` call.
    pub fn step(self, input: &ChooserInput<'_>) -> Result<(Self, ChooserStep), ModuleChooserError> {
        if input.activity.0 & activity_flags::CHECK_ABORT != 0 {
            return Ok((self, ChooserStep::Abort));
        }

        let fittings = Fittings::from_view(input.ship)?;
        let here = fittings.piece_at(self.cursor.kind(), self.item)?;

        if !self.opened {
            let chooser = Self {
                opened: true,
                ..self
            };
            return Ok((chooser, ChooserStep::Open { shown: here }));
        }

        let pulses = input.pulses;
        let costs = ModuleCosts::from(input.ship.module_cost);
        if pulses.select {
            self.select(&fittings, &costs, input.ship, here)
        } else if pulses.cancel {
            Ok(self.cancel())
        } else if pulses.left || pulses.right || pulses.up || pulses.down {
            let bomb_installed = input.chmmr_bomb_state == CHMMR_BOMB_INSTALLED;
            self.move_cursor(pulses, &fittings, &costs, bomb_installed)
        } else {
            Ok((self, ChooserStep::Idle))
        }
    }

    fn with_cursor(self, cursor: Cursor) -> Self {
        Self { cursor, ..self }
    }

    /// Select (`outfit.c:241-358`).
    fn select(
        self,
        fittings: &Fittings,
        costs: &ModuleCosts,
        ship: &SisView,
        here: Option<ModuleType>,
    ) -> Result<(Self, ChooserStep), ModuleChooserError> {
        let kind = self.cursor.kind();
        match (self.cursor, here) {
            (Cursor::Offer(offered), None) => {
                let price = module_price(costs, offered);
                if let ModuleCheck::Blocked(block) = check_module_purchase(ship.res_units, price) {
                    return Ok((self, ChooserStep::Refused(block)));
                }
                let transaction = Transaction {
                    placement: fittings.placement(kind, self.item, true),
                    change: Change::Install {
                        module: offered,
                        price,
                    },
                };
                let next = self.with_cursor(Cursor::Slots(kind));
                Ok((next, ChooserStep::Apply(transaction)))
            }
            (Cursor::Offer(offered), Some(_)) => Err(ModuleChooserError::OfferedSlotOccupied {
                offered,
                kind,
                slot: self.item,
            }),
            (Cursor::Slots(_), None) => {
                let piece = kind.first_offer();
                Ok((
                    self.with_cursor(Cursor::Offer(piece)),
                    ChooserStep::Offer { piece },
                ))
            }
            (Cursor::Slots(_), Some(module)) => {
                if kind == SlotKind::Module {
                    let load = ShipLoad {
                        crew_enlisted: ship.crew_enlisted,
                        total_element_mass: ship.total_element_mass,
                        fuel_on_board: ship.fuel_on_board,
                    };
                    let check =
                        check_module_removal(&fittings.modules, usize::from(self.item), load)?;
                    if let ModuleCheck::Blocked(block) = check {
                        return Ok((self, ChooserStep::Refused(block)));
                    }
                }
                let transaction = Transaction {
                    placement: fittings.placement(kind, self.item, false),
                    change: Change::Remove {
                        module,
                        refund: module_price(costs, module),
                    },
                };
                Ok((self, ChooserStep::Apply(transaction)))
            }
        }
    }

    /// Cancel (`outfit.c:358-410`): offering goes back to the slots, and the
    /// slots go back to the Outfit menu.
    fn cancel(self) -> (Self, ChooserStep) {
        match self.cursor {
            Cursor::Offer(piece) => (
                self.with_cursor(Cursor::Slots(SlotKind::holding(piece))),
                ChooserStep::BackToSlots,
            ),
            Cursor::Slots(_) => (self, ChooserStep::Leave),
        }
    }

    /// The motion branch (`outfit.c:411-535`).
    fn move_cursor(
        self,
        pulses: MenuPulses,
        fittings: &Fittings,
        costs: &ModuleCosts,
        bomb_installed: bool,
    ) -> Result<(Self, ChooserStep), ModuleChooserError> {
        match self.cursor {
            Cursor::Offer(offered) => {
                let piece = self.next_offer(offered, pulses, costs)?;
                if piece == offered {
                    return Ok((self, ChooserStep::Idle));
                }
                Ok((
                    self.with_cursor(Cursor::Offer(piece)),
                    ChooserStep::ShowOffer { piece },
                ))
            }
            Cursor::Slots(kind) => {
                let (kind_after, item_after) = self.next_slot(kind, pulses, bomb_installed);
                if kind_after == kind && item_after == self.item {
                    return Ok((self, ChooserStep::Idle));
                }
                let chooser = Self {
                    cursor: Cursor::Slots(kind_after),
                    item: item_after,
                    ..self
                };
                let shown = fittings.piece_at(kind_after, item_after)?;
                Ok((chooser, ChooserStep::Move { shown }))
            }
        }
    }

    /// Up and down change the kind of slot, left and right the slot within
    /// the kind.
    fn next_slot(self, kind: SlotKind, pulses: MenuPulses, bomb_installed: bool) -> (SlotKind, u8) {
        let (first, last) = kind.item_range(bomb_installed);
        if pulses.up || pulses.down {
            let mut kind = if pulses.up {
                kind.previous()
            } else {
                kind.next()
            };
            // The bomb takes the lander berths' place in the cycle.
            if bomb_installed && kind == SlotKind::Lander {
                kind = if pulses.up {
                    SlotKind::Module
                } else {
                    SlotKind::Drive
                };
            }
            return (kind, kind.item_range(bomb_installed).0);
        }
        let item = if pulses.left {
            if self.item == first {
                last
            } else {
                self.item - 1
            }
        } else if self.item == last {
            first
        } else {
            self.item + 1
        };
        (kind, item)
    }

    /// Left and up step back through the modules on offer, right and down
    /// forward, skipping modules that cost nothing (not for sale yet) and
    /// weapons for a slot that cannot hold one.
    fn next_offer(
        self,
        offered: ModuleType,
        pulses: MenuPulses,
        costs: &ModuleCosts,
    ) -> Result<ModuleType, ModuleChooserError> {
        let (first, last) = if (offered as u8) < FIRST_PURCHASE_MODULE as u8 {
            (offered as u8, offered as u8)
        } else {
            (FIRST_PURCHASE_MODULE as u8, LAST_PURCHASE_MODULE as u8)
        };
        let backward = pulses.left || pulses.up;
        let mut candidate = offered as u8;
        for _ in first..=last {
            candidate = if backward {
                if candidate == first {
                    last
                } else {
                    candidate - 1
                }
            } else if candidate == last {
                first
            } else {
                candidate + 1
            };
            let module = ModuleType::from_u8(candidate)
                .expect("offered pieces stay within the module numbers");
            let is_weapon =
                (ModuleType::GunWeapon as u8..=ModuleType::CannonWeapon as u8).contains(&candidate);
            let forbidden_weapon = is_weapon && WEAPON_EXCLUDED_SLOTS.contains(&self.item);
            if module_price(costs, module) != 0 && !forbidden_weapon {
                return Ok(module);
            }
        }
        Err(ModuleChooserError::NothingToOffer {
            first: ModuleType::from_u8(first).expect("range starts at a module"),
            last: ModuleType::from_u8(last).expect("range ends at a module"),
            slot: self.item,
        })
    }
}

/// The rectangle the slot at `item` of `kind` flashes, in space context
/// coordinates (`InitFlash`, `outfit.c:488-531`).
#[must_use]
pub fn slot_flash_rect(kind: SlotKind, item: u8) -> Rect {
    let (corner, (width, height), pitch) = match kind {
        SlotKind::Lander => (
            Point {
                x: LANDER_X,
                y: LANDER_Y,
            },
            LANDER_FLASH_SIZE,
            LANDER_WIDTH,
        ),
        SlotKind::Drive => (DRIVE_TOP, DRIVE_FLASH_SIZE, SHIP_PIECE_OFFSET),
        SlotKind::Jet => (JET_TOP, JET_FLASH_SIZE, SHIP_PIECE_OFFSET),
        SlotKind::Module => (MODULE_TOP, MODULE_FLASH_SIZE, SHIP_PIECE_OFFSET),
    };
    Rect {
        corner: Point {
            x: corner.x - 1 + pitch * i16::from(item),
            y: corner.y - 1,
        },
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::starbase::new_game_reference::{NewGameFlagship, PieceNumbers};
    use crate::starbase::sis_modules::EMPTY_MODULE_SLOT;

    const ALL_KINDS: [SlotKind; 4] = [
        SlotKind::Drive,
        SlotKind::Jet,
        SlotKind::Module,
        SlotKind::Lander,
    ];

    fn new_game() -> SisView {
        NewGameFlagship::from_initializer().sis_view()
    }

    fn input<'a>(ship: &'a SisView, pulses: MenuPulses) -> ChooserInput<'a> {
        ChooserInput {
            pulses,
            activity: ActivityValue::new(0),
            ship,
            chmmr_bomb_state: 0,
        }
    }

    fn opened(ship: &SisView) -> ModuleChooser {
        let (chooser, step) = ModuleChooser::new(0)
            .step(&input(ship, MenuPulses::default()))
            .expect("the new game hull is valid");
        assert!(matches!(step, ChooserStep::Open { .. }));
        chooser
    }

    fn pressing(set: impl FnOnce(&mut MenuPulses)) -> MenuPulses {
        let mut pulses = MenuPulses::default();
        set(&mut pulses);
        pulses
    }

    #[test]
    fn empty_slot_bytes_are_the_native_sentinels() {
        assert_eq!(SlotKind::Drive.empty_piece(), 20);
        assert_eq!(SlotKind::Jet.empty_piece(), 21);
        assert_eq!(SlotKind::Module.empty_piece(), 22);
        assert_eq!(SlotKind::Module.empty_piece(), EMPTY_MODULE_SLOT);
        assert_eq!(SlotKind::Lander.empty_piece(), 23);
        let pieces = PieceNumbers::from_schema();
        assert_eq!(
            u32::from(SlotKind::Drive.empty_piece()),
            pieces.get("NUM_MODULES")
        );
    }

    #[test]
    fn slot_counts_match_the_c_arrays() {
        assert_eq!(u32::from(SlotKind::Drive.slot_count()), 11);
        assert_eq!(u32::from(SlotKind::Jet.slot_count()), 8);
        assert_eq!(u32::from(SlotKind::Module.slot_count()), 16);
        assert_eq!(u32::from(SlotKind::Lander.slot_count()), 10);
    }

    #[test]
    fn the_first_offer_follows_the_c_arithmetic_on_the_empty_byte() {
        // outfit.c:305-309: new_slot_piece -= EMPTY_SLOT - 1; if it is past
        // CREW_POD it becomes PLANET_LANDER.
        for kind in ALL_KINDS {
            let mut piece = kind.empty_piece() - (EMPTY_SLOT - 1);
            if piece > ModuleType::CrewPod as u8 {
                piece = ModuleType::PlanetLander as u8;
            }
            assert_eq!(Some(kind.first_offer()), ModuleType::from_u8(piece));
        }
    }

    #[test]
    fn the_kind_an_installed_piece_returns_to_follows_the_c_arithmetic() {
        // outfit.c:392-398: CurState += EMPTY_SLOT - 1; below EMPTY_SLOT it
        // becomes EMPTY_SLOT + 3, above EMPTY_SLOT + 2 it becomes
        // EMPTY_SLOT + 2.
        let purchasable = PieceNumbers::from_schema().get("NUM_PURCHASE_MODULES");
        for raw in 0..purchasable as u8 {
            let mut state = raw + EMPTY_SLOT - 1;
            if state < EMPTY_SLOT {
                state = EMPTY_SLOT + 3;
            } else if state > EMPTY_SLOT + 2 {
                state = EMPTY_SLOT + 2;
            }
            let piece = ModuleType::from_u8(raw).unwrap();
            assert_eq!(SlotKind::holding(piece).empty_piece(), state, "{raw}");
        }
    }

    #[test]
    fn the_bomb_starts_the_module_cursor_past_the_bomb_slots() {
        assert_eq!(ModuleChooser::new(0).item(), 0);
        assert_eq!(ModuleChooser::new(2).item(), 0);
        assert_eq!(ModuleChooser::new(3).item(), 10);
    }

    #[test]
    fn abort_is_tested_before_the_ship_is_read() {
        let mut ship = new_game();
        ship.drive_slots[0] = 99;
        let abort = ChooserInput {
            activity: ActivityValue::new(activity_flags::CHECK_ABORT),
            ..input(&ship, MenuPulses::default())
        };
        assert_eq!(
            ModuleChooser::new(0).step(&abort).unwrap().1,
            ChooserStep::Abort
        );
    }

    #[test]
    fn selecting_a_filled_lander_berth_removes_one_lander() {
        let ship = new_game();
        let mut chooser = opened(&ship);
        chooser.cursor = Cursor::Slots(SlotKind::Lander);
        chooser.item = 0;
        let (_, step) = chooser
            .step(&input(&ship, pressing(|p| p.select = true)))
            .unwrap();
        let ChooserStep::Apply(transaction) = step else {
            panic!("expected a removal, got {step:?}");
        };
        assert_eq!(transaction.placement, Placement::Landers { count_after: 0 });
        assert_eq!(
            transaction.change,
            Change::Remove {
                module: ModuleType::PlanetLander,
                refund: 500
            }
        );
        assert_eq!(transaction.new_piece(), 23);
        assert_eq!(transaction.old_piece(), 0);
        assert_eq!(transaction.res_units_delta(), 500);
    }

    #[test]
    fn a_transaction_reports_both_slot_bytes_and_the_signed_ru() {
        let install = Transaction {
            placement: Placement::Module(4),
            change: Change::Install {
                module: ModuleType::FuelTank,
                price: 500,
            },
        };
        assert_eq!(install.new_piece(), ModuleType::FuelTank as u8);
        assert_eq!(install.old_piece(), 22);
        assert_eq!(install.res_units_delta(), -500);
        assert_eq!(install.shown(), Some(ModuleType::FuelTank));

        let remove = Transaction {
            placement: Placement::Drive(5),
            change: Change::Remove {
                module: ModuleType::FusionThruster,
                refund: 500,
            },
        };
        assert_eq!(remove.new_piece(), 20);
        assert_eq!(remove.old_piece(), ModuleType::FusionThruster as u8);
        assert_eq!(remove.res_units_delta(), 500);
        assert_eq!(remove.shown(), None);
    }

    #[test]
    fn an_open_offer_on_a_filled_slot_is_an_internal_error() {
        let ship = new_game();
        let mut chooser = opened(&ship);
        // Slot 1 holds the fuel tank.
        chooser.cursor = Cursor::Offer(ModuleType::CrewPod);
        chooser.item = 1;
        let result = chooser.step(&input(&ship, pressing(|p| p.select = true)));
        assert_eq!(
            result.unwrap_err(),
            ModuleChooserError::OfferedSlotOccupied {
                offered: ModuleType::CrewPod,
                kind: SlotKind::Module,
                slot: 1
            }
        );
    }

    #[test]
    fn foreign_slot_bytes_are_named_errors() {
        let mut ship = new_game();
        ship.jet_slots[2] = 1;
        assert_eq!(
            Fittings::from_view(&ship).unwrap_err(),
            ModuleChooserError::JetSlot { index: 2, value: 1 }
        );

        let mut ship = new_game();
        ship.drive_slots[4] = 255;
        assert_eq!(
            Fittings::from_view(&ship).unwrap_err(),
            ModuleChooserError::DriveSlot {
                index: 4,
                value: 255
            }
        );

        let mut ship = new_game();
        ship.num_landers = MAX_LANDERS + 1;
        assert_eq!(
            Fittings::from_view(&ship).unwrap_err(),
            ModuleChooserError::TooManyLanders {
                count: MAX_LANDERS + 1
            }
        );

        let mut ship = new_game();
        ship.module_slots[0] = 23;
        assert!(matches!(
            Fittings::from_view(&ship),
            Err(ModuleChooserError::ModuleSlots(_))
        ));
    }

    #[test]
    fn the_flash_rectangle_steps_by_the_slot_pitch() {
        let first = slot_flash_rect(SlotKind::Module, 0);
        let fifth = slot_flash_rect(SlotKind::Module, 5);
        assert_eq!(fifth.corner.x - first.corner.x, 5 * 12);
        assert_eq!(fifth.corner.y, first.corner.y);
        let lander = slot_flash_rect(SlotKind::Lander, 2);
        assert_eq!(lander.corner.x, 23 + 2 * 15);
    }
}
