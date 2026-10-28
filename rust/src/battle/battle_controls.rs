// Battle input controls — port of C battlecontrols.c
// Defines BattleInputHandlers vtables, InputContext base struct, and
// context constructors. Rust handler functions are referenced through the
// C ABI.

use crate::memory::{rust_hfree, rust_hmalloc};
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
        crate::battle::ai_entry::computer_intelligence(
            ctx.cast::<crate::ships::native::abi::ComputerInputContext>(),
            ship.cast::<crate::ships::native::abi::STARSHIP>(),
        )
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
        crate::battle::battle_native::frameInputHuman(
            ctx.cast::<crate::ships::native::abi::HumanInputContext>(),
            ship.cast::<crate::ships::native::abi::STARSHIP>(),
        )
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

/// C: `InputContext *PlayerInput[NUM_PLAYERS]`
#[no_mangle]
pub static mut PlayerInput: [*mut InputContext; NUM_PLAYERS] =
    [std::ptr::null_mut(), std::ptr::null_mut()];

#[cfg(not(test))]
use crate::supermelee::native::battle_picker::live::selectShipNetwork;

#[cfg(not(test))]
use crate::battle::tactrans_native::engine::battleEndReadyNetwork;

/// C: `void InputContext_init(InputContext *context, BattleInputHandlers *handlers, COUNT playerNr)`
#[no_mangle]
/// # Safety
/// `context` must point to aligned, writable InputContext storage.
/// `handlers` must remain valid while the initialized context uses it.
pub unsafe extern "C" fn InputContext_init(
    context: *mut InputContext,
    handlers: *const BattleInputHandlers,
    player_nr: Count,
) {
    unsafe {
        (*context).handlers = handlers;
        (*context).player_nr = player_nr;
    }
}

/// C: `void InputContext_delete(InputContext *context)` — frees with HFree
#[no_mangle]
pub extern "C" fn InputContext_delete(context: *mut InputContext) {
    unsafe {
        rust_hfree(context as *mut c_void);
    }
}

/// C: `ComputerInputContext *ComputerInputContext_new(COUNT playerNr)`
#[no_mangle]
pub extern "C" fn ComputerInputContext_new(player_nr: Count) -> *mut ComputerInputContext {
    unsafe {
        let result =
            rust_hmalloc(std::mem::size_of::<ComputerInputContext>()) as *mut ComputerInputContext;
        if !result.is_null() {
            InputContext_init(
                &mut (*result).base as *mut InputContext,
                &COMPUTER_INPUT_HANDLERS as *const _,
                player_nr,
            );
        }
        result
    }
}

/// C: `HumanInputContext *HumanInputContext_new(COUNT playerNr)`
#[no_mangle]
pub extern "C" fn HumanInputContext_new(player_nr: Count) -> *mut HumanInputContext {
    unsafe {
        let result =
            rust_hmalloc(std::mem::size_of::<HumanInputContext>()) as *mut HumanInputContext;
        if !result.is_null() {
            InputContext_init(
                &mut (*result).base as *mut InputContext,
                &HUMAN_INPUT_HANDLERS as *const _,
                player_nr,
            );
        }
        result
    }
}

/// C: `NetworkInputContext *NetworkInputContext_new(COUNT playerNr)`
#[no_mangle]
pub extern "C" fn NetworkInputContext_new(player_nr: Count) -> *mut NetworkInputContext {
    unsafe {
        let result =
            rust_hmalloc(std::mem::size_of::<NetworkInputContext>()) as *mut NetworkInputContext;
        if result.is_null() {
            return result;
        }

        std::ptr::addr_of_mut!((*result).net).write(None);
        InputContext_init(
            &mut (*result).base as *mut InputContext,
            &NETWORK_INPUT_HANDLERS as *const _,
            player_nr,
        );
        result
    }
}

unsafe extern "C" fn unsafe_frame_input_network(
    ctx: *mut InputContext,
    _ship: *mut c_void,
) -> BattleInputState {
    let context = ctx.cast::<NetworkInputContext>();
    (*context)
        .net
        .as_ref()
        .unwrap()
        .remote_input((*ctx).player_nr as usize)
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

unsafe extern "C" fn delete_network_context(context: *mut InputContext) {
    std::ptr::drop_in_place(std::ptr::addr_of_mut!(
        (*context.cast::<NetworkInputContext>()).net
    ));
    InputContext_delete(context);
}
