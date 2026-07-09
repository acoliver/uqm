// Battle input controls — port of C battlecontrols.c
// Defines BattleInputHandlers vtables, InputContext base struct, and
// context constructors. Handler functions (computer_intelligence,
// frameInputHuman, selectShip*, battleEndReady*) live in other C files
// (intel.c, battle.c, tactrans.c, pickmele.c) and are referenced via
// extern C function pointers.

use std::os::raw::{c_int, c_void};

/// COUNT = u16
pub type Count = u16;

/// BATTLE_INPUT_STATE = DWORD = u32
pub type BattleInputState = u32;

/// BOOLEAN = c_int
pub type Boolean = c_int;

// NUM_PLAYERS is a C constant (#define or enum)
pub const NUM_PLAYERS: usize = 2;

// ---------------------------------------------------------------------------
// Function pointer types (matching C typedefs)
// ---------------------------------------------------------------------------

/// C: `typedef BATTLE_INPUT_STATE (*BattleFrameInputFunction)(InputContext*, STARSHIP*)`
pub type BattleFrameInputFn = Option<
    unsafe extern "C" fn(*mut InputContext, *mut c_void) -> BattleInputState,
>;

/// C: `typedef BOOLEAN (*SelectShipFunction)(InputContext*, GETMELEE_STATE*)`
pub type SelectShipFn = Option<
    unsafe extern "C" fn(*mut InputContext, *mut c_void) -> Boolean,
>;

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
}

// ---------------------------------------------------------------------------
// Extern C handler functions (defined in other C files)
// ---------------------------------------------------------------------------

extern "C" {
    fn computer_intelligence(
        context: *mut c_void,
        starship: *mut c_void,
    ) -> BattleInputState;

    fn frameInputHuman(
        context: *mut c_void,
        starship: *mut c_void,
    ) -> BattleInputState;

    fn battleEndReadyComputer(context: *mut c_void) -> bool;
    fn battleEndReadyHuman(context: *mut c_void) -> bool;

    // selectShipComputer/Human/Network are in supermelee/pickmele.c
    fn selectShipComputer(context: *mut c_void, gms: *mut c_void) -> Boolean;
    fn selectShipHuman(context: *mut c_void, gms: *mut c_void) -> Boolean;
}

// ---------------------------------------------------------------------------
// Static vtables — matching C's global BattleInputHandlers structs
// ---------------------------------------------------------------------------

/// C: `BattleInputHandlers ComputerInputHandlers`
#[no_mangle]
pub static ComputerInputHandlers: BattleInputHandlers = BattleInputHandlers {
    frame_input: Some(unsafe_frame_input_computer),
    select_ship: Some(unsafe_select_ship_computer),
    battle_end_ready: Some(unsafe_battle_end_ready_computer),
    delete_context: Some(InputContext_delete),
};

/// C: `BattleInputHandlers HumanInputHandlers`
#[no_mangle]
pub static HumanInputHandlers: BattleInputHandlers = BattleInputHandlers {
    frame_input: Some(unsafe_frame_input_human),
    select_ship: Some(unsafe_select_ship_human),
    battle_end_ready: Some(unsafe_battle_end_ready_human),
    delete_context: Some(InputContext_delete),
};

// Trampoline functions to match the exact C function pointer signatures.
// The extern "C" functions have different parameter types (e.g.
// HumanInputContext* vs InputContext*) so we need thin wrappers.

unsafe extern "C" fn unsafe_frame_input_computer(
    ctx: *mut InputContext,
    ship: *mut c_void,
) -> BattleInputState {
    computer_intelligence(ctx as *mut c_void, ship)
}

unsafe extern "C" fn unsafe_frame_input_human(
    ctx: *mut InputContext,
    ship: *mut c_void,
) -> BattleInputState {
    frameInputHuman(ctx as *mut c_void, ship)
}

unsafe extern "C" fn unsafe_select_ship_computer(
    ctx: *mut InputContext,
    gms: *mut c_void,
) -> Boolean {
    selectShipComputer(ctx as *mut c_void, gms)
}

unsafe extern "C" fn unsafe_select_ship_human(
    ctx: *mut InputContext,
    gms: *mut c_void,
) -> Boolean {
    selectShipHuman(ctx as *mut c_void, gms)
}

unsafe extern "C" fn unsafe_battle_end_ready_computer(ctx: *mut InputContext) -> bool {
    battleEndReadyComputer(ctx as *mut c_void)
}

unsafe extern "C" fn unsafe_battle_end_ready_human(ctx: *mut InputContext) -> bool {
    battleEndReadyHuman(ctx as *mut c_void)
}

// ---------------------------------------------------------------------------
// Public API functions (matching C signatures)
// ---------------------------------------------------------------------------

/// C: `InputContext *PlayerInput[NUM_PLAYERS]`
#[no_mangle]
pub static mut PlayerInput: [*mut InputContext; NUM_PLAYERS] =
    [std::ptr::null_mut(), std::ptr::null_mut()];

extern "C" {
    fn rust_hmalloc(size: usize) -> *mut c_void;
    fn rust_hfree(ptr: *mut c_void);
}

/// C: `void InputContext_init(InputContext *context, BattleInputHandlers *handlers, COUNT playerNr)`
#[no_mangle]
pub extern "C" fn InputContext_init(
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
        let result = rust_hmalloc(std::mem::size_of::<ComputerInputContext>())
            as *mut ComputerInputContext;
        if !result.is_null() {
            InputContext_init(
                &mut (*result).base as *mut InputContext,
                &ComputerInputHandlers as *const _,
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
        let result = rust_hmalloc(std::mem::size_of::<HumanInputContext>())
            as *mut HumanInputContext;
        if !result.is_null() {
            InputContext_init(
                &mut (*result).base as *mut InputContext,
                &HumanInputHandlers as *const _,
                player_nr,
            );
        }
        result
    }
}

// ---------------------------------------------------------------------------
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