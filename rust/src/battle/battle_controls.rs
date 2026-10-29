// Battle input controls — port of C battlecontrols.c
// Defines BattleInputHandlers vtables, InputContext base struct, and
// context constructors. Rust handler functions are referenced through the
// C ABI.

use std::os::raw::c_void;

/// COUNT = u16
pub type Count = u16;

/// BATTLE_INPUT_STATE = UBYTE = u8
pub type BattleInputState = u8;

/// BOOLEAN = unsigned int = u32
pub type Boolean = u32;

// NUM_PLAYERS is a C constant (#define or enum)
pub const NUM_PLAYERS: usize = 2;

// ---------------------------------------------------------------------------
// Function pointer types (matching C typedefs)
// ---------------------------------------------------------------------------

/// C: `typedef BATTLE_INPUT_STATE (*BattleFrameInputFunction)(InputContext*, STARSHIP*)`
pub type BattleFrameInputFn =
    Option<unsafe extern "C" fn(*mut InputContext, *mut c_void) -> BattleInputState>;

/// C: `typedef BOOLEAN (*SelectShipFunction)(InputContext*, GETMELEE_STATE*)`
pub type SelectShipFn = Option<unsafe extern "C" fn(*mut InputContext, *mut c_void) -> Boolean>;

/// C: `typedef bool (*BattleEndReadyFunction)(InputContext*)`
pub type BattleEndReadyFn = Option<unsafe extern "C" fn(*mut InputContext) -> bool>;

/// C: `typedef void (*DeleteInputContextFunction)(InputContext*)`
pub type DeleteInputContextFn = Option<unsafe extern "C" fn(*mut InputContext)>;

// ---------------------------------------------------------------------------
// C-compatible structs (#[repr(C)])
// ---------------------------------------------------------------------------

/// C: `struct BattleInputHandlers { BattleFrameInputFunction frameInput;
///   SelectShipFunction selectShip; BattleEndReadyFunction battleEndReady;
///   DeleteInputContextFunction deleteContext; }`
#[repr(C)]
pub struct BattleInputHandlers {
    pub frame_input: BattleFrameInputFn,
    pub select_ship: SelectShipFn,
    pub battle_end_ready: BattleEndReadyFn,
    pub delete_context: DeleteInputContextFn,
}

/// C: INPUT_CONTEXT_COMMON macro expands to:
///   BattleInputHandlers *handlers; COUNT playerNr;
/// InputContext is the base "class" — all *InputContext structs start
/// with these two fields.
#[repr(C)]
pub struct InputContext {
    pub handlers: *const BattleInputHandlers,
    pub player_nr: Count,
}

/// ComputerInputContext = InputContext + (TODO: RNG context for AI)
#[repr(C)]
pub struct ComputerInputContext {
    pub base: InputContext,
}

/// HumanInputContext = InputContext (no extra fields)
#[repr(C)]
pub struct HumanInputContext {
    pub base: InputContext,
}

/// NetworkInputContext = InputContext (no extra fields currently)
#[repr(C)]
pub struct NetworkInputContext {
    pub base: InputContext,
    pub net: Option<std::rc::Rc<dyn crate::engine::battle::net::BattleNet>>,
}

// ---------------------------------------------------------------------------
// The battle-picker Rust implementations are private to their native module.
// ---------------------------------------------------------------------------

#[cfg(not(test))]
use crate::supermelee::native::battle_picker::live::selectShipComputer;
#[cfg(not(test))]
use crate::supermelee::native::battle_picker::live::selectShipHuman;

// ---------------------------------------------------------------------------
// Static vtables — matching C's global BattleInputHandlers structs
// ---------------------------------------------------------------------------

/// C: `BattleInputHandlers COMPUTER_INPUT_HANDLERS`
pub static COMPUTER_INPUT_HANDLERS: BattleInputHandlers = BattleInputHandlers {
    frame_input: Some(unsafe_frame_input_computer),
    select_ship: Some(unsafe_select_ship_computer),
    battle_end_ready: Some(unsafe_battle_end_ready_computer),
    delete_context: Some(InputContext_delete),
};

/// C: `BattleInputHandlers HUMAN_INPUT_HANDLERS`
pub static HUMAN_INPUT_HANDLERS: BattleInputHandlers = BattleInputHandlers {
    frame_input: Some(unsafe_frame_input_human),
    select_ship: Some(unsafe_select_ship_human),
    battle_end_ready: Some(unsafe_battle_end_ready_human),
    delete_context: Some(InputContext_delete),
};
/// C: `BattleInputHandlers NETWORK_INPUT_HANDLERS`
pub static NETWORK_INPUT_HANDLERS: BattleInputHandlers = BattleInputHandlers {
    frame_input: Some(unsafe_frame_input_network),
    select_ship: Some(unsafe_select_ship_network),
    battle_end_ready: Some(unsafe_battle_end_ready_network),
    delete_context: Some(delete_network_context),
};

// Trampoline functions to match the exact C function pointer signatures.
// The extern "C" functions have different parameter types (e.g.
// HumanInputContext* vs InputContext*) so we need thin wrappers.

unsafe extern "C" fn unsafe_frame_input_computer(
    ctx: *mut InputContext,
    ship: *mut c_void,
) -> BattleInputState {
    #[cfg(not(test))]
    {
        crate::battle::context_access::reader_legacy::computer_frame(ctx, ship)
    }
    #[cfg(test)]
    {
        handler_stubs::computer_intelligence(
            ctx.cast::<crate::ships::native::abi::ComputerInputContext>(),
            ship.cast::<crate::ships::native::abi::STARSHIP>(),
        )
    }
}

unsafe extern "C" fn unsafe_frame_input_human(
    ctx: *mut InputContext,
    ship: *mut c_void,
) -> BattleInputState {
    #[cfg(not(test))]
    {
        crate::battle::context_access::reader_legacy::human_frame(ctx, ship)
    }
    #[cfg(test)]
    {
        handler_stubs::frame_input_human(
            ctx.cast::<crate::ships::native::abi::HumanInputContext>(),
            ship.cast::<crate::ships::native::abi::STARSHIP>(),
        )
    }
}

unsafe extern "C" fn unsafe_select_ship_computer(
    ctx: *mut InputContext,
    gms: *mut c_void,
) -> Boolean {
    #[cfg(not(test))]
    {
        selectShipComputer(
            ctx.cast::<crate::ships::native::abi::ComputerInputContext>(),
            gms.cast::<crate::ships::native::abi::GETMELEE_STATE>(),
        )
    }
    #[cfg(test)]
    {
        handler_stubs::select_ship_computer(
            ctx.cast::<crate::ships::native::abi::ComputerInputContext>(),
            gms.cast::<crate::ships::native::abi::GETMELEE_STATE>(),
        )
    }
}

unsafe extern "C" fn unsafe_select_ship_human(ctx: *mut InputContext, gms: *mut c_void) -> Boolean {
    #[cfg(not(test))]
    {
        selectShipHuman(
            ctx.cast::<crate::ships::native::abi::HumanInputContext>(),
            gms.cast::<crate::ships::native::abi::GETMELEE_STATE>(),
        )
    }
    #[cfg(test)]
    {
        handler_stubs::select_ship_human(
            ctx.cast::<crate::ships::native::abi::HumanInputContext>(),
            gms.cast::<crate::ships::native::abi::GETMELEE_STATE>(),
        )
    }
}

unsafe extern "C" fn unsafe_battle_end_ready_computer(ctx: *mut InputContext) -> bool {
    #[cfg(not(test))]
    {
        crate::battle::tactrans_native::battleEndReadyComputer(
            ctx.cast::<crate::ships::native::abi::ComputerInputContext>(),
        )
    }
    #[cfg(test)]
    {
        handler_stubs::battle_end_ready_computer(
            ctx.cast::<crate::ships::native::abi::ComputerInputContext>(),
        )
    }
}

unsafe extern "C" fn unsafe_battle_end_ready_human(ctx: *mut InputContext) -> bool {
    #[cfg(not(test))]
    {
        crate::battle::tactrans_native::battleEndReadyHuman(
            ctx.cast::<crate::ships::native::abi::HumanInputContext>(),
        )
    }
    #[cfg(test)]
    {
        handler_stubs::battle_end_ready_human(
            ctx.cast::<crate::ships::native::abi::HumanInputContext>(),
        )
    }
}

// ---------------------------------------------------------------------------
// Public API functions (matching C signatures)
// ---------------------------------------------------------------------------

#[cfg(not(test))]
use crate::supermelee::native::battle_picker::live::selectShipNetwork;

#[cfg(not(test))]
use crate::battle::tactrans_native::engine::battleEndReadyNetwork;

/// Release a human or computer context, both of which contain only the base record.
///
/// # Safety
/// `context` must be null or an unconsumed allocation from the human/computer constructors.
/// Network contexts must use their vtable destructor to drop the session handle.
#[no_mangle]
pub unsafe extern "C" fn InputContext_delete(context: *mut InputContext) {
    unsafe {
        if !context.is_null() {
            drop(Box::from_raw(context));
        }
    }
}

/// C: `ComputerInputContext *ComputerInputContext_new(COUNT playerNr)`
#[no_mangle]
pub extern "C" fn ComputerInputContext_new(player_nr: Count) -> *mut ComputerInputContext {
    Box::into_raw(Box::new(ComputerInputContext {
        base: InputContext {
            handlers: &COMPUTER_INPUT_HANDLERS,
            player_nr,
        },
    }))
}

#[no_mangle]
pub extern "C" fn HumanInputContext_new(player_nr: Count) -> *mut HumanInputContext {
    Box::into_raw(Box::new(HumanInputContext {
        base: InputContext {
            handlers: &HUMAN_INPUT_HANDLERS,
            player_nr,
        },
    }))
}

#[no_mangle]
pub extern "C" fn NetworkInputContext_new(player_nr: Count) -> *mut NetworkInputContext {
    Box::into_raw(Box::new(NetworkInputContext {
        base: InputContext {
            handlers: &NETWORK_INPUT_HANDLERS,
            player_nr,
        },
        net: None,
    }))
}

unsafe extern "C" fn unsafe_frame_input_network(
    ctx: *mut InputContext,
    _ship: *mut c_void,
) -> BattleInputState {
    // SAFETY: the network vtable is installed only on a live NetworkInputContext.
    let context = unsafe { &*ctx.cast::<NetworkInputContext>() };
    context
        .net
        .as_ref()
        .unwrap_or_else(|| panic!("network input has no session"))
        .remote_input(context.base.player_nr as usize)
}

unsafe extern "C" fn unsafe_select_ship_network(
    ctx: *mut InputContext,
    gms: *mut c_void,
) -> Boolean {
    #[cfg(not(test))]
    {
        selectShipNetwork(ctx.cast(), gms.cast())
    }
    #[cfg(test)]
    {
        let _ = (ctx, gms);
        panic!("unit test entered live network ship selection")
    }
}

unsafe extern "C" fn unsafe_battle_end_ready_network(ctx: *mut InputContext) -> bool {
    #[cfg(not(test))]
    {
        battleEndReadyNetwork(ctx.cast())
    }
    #[cfg(test)]
    {
        let _ = ctx;
        panic!("unit test entered live network battle completion")
    }
}

// ---------------------------------------------------------------------------
unsafe extern "C" fn delete_network_context(context: *mut InputContext) {
    // SAFETY: this vtable entry consumes the Box allocated by NetworkInputContext_new.
    unsafe { drop(Box::from_raw(context.cast::<NetworkInputContext>())) };
}

#[cfg(test)]
mod handler_stubs {
    use super::{BattleInputState, Boolean};
    use crate::ships::native::abi::{
        ComputerInputContext, HumanInputContext, GETMELEE_STATE, STARSHIP,
    };

    pub unsafe extern "C" fn computer_intelligence(
        _context: *mut ComputerInputContext,
        _ship: *mut STARSHIP,
    ) -> BattleInputState {
        0
    }

    pub unsafe extern "C" fn frame_input_human(
        _context: *mut HumanInputContext,
        _ship: *mut STARSHIP,
    ) -> BattleInputState {
        0
    }

    pub unsafe extern "C" fn select_ship_computer(
        _context: *mut ComputerInputContext,
        _gms: *mut GETMELEE_STATE,
    ) -> Boolean {
        0
    }

    pub unsafe extern "C" fn select_ship_human(
        _context: *mut HumanInputContext,
        _gms: *mut GETMELEE_STATE,
    ) -> Boolean {
        0
    }

    pub unsafe extern "C" fn battle_end_ready_computer(
        _context: *mut ComputerInputContext,
    ) -> bool {
        true
    }

    pub unsafe extern "C" fn battle_end_ready_human(_context: *mut HumanInputContext) -> bool {
        true
    }
}

// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, not(feature = "audio_heart")))]
mod tests {
    use super::*;

    #[test]
    fn test_handlers_struct_layout() {
        // BattleInputHandlers must be 4 function pointers
        assert_eq!(
            std::mem::size_of::<BattleInputHandlers>(),
            std::mem::size_of::<usize>() * 4
        );
    }

    #[test]
    fn test_input_context_layout() {
        // InputContext must be 2 pointers (16 bytes on 64-bit)
        assert_eq!(
            std::mem::size_of::<InputContext>(),
            std::mem::size_of::<usize>() * 2
        );
    }

    #[test]
    fn test_player_input_array_size() {
        assert_eq!(NUM_PLAYERS, 2);
    }

    #[test]
    fn test_computer_context_same_size_as_base() {
        // ComputerInputContext has no extra fields beyond InputContext
        assert_eq!(
            std::mem::size_of::<ComputerInputContext>(),
            std::mem::size_of::<InputContext>()
        );
    }
}
