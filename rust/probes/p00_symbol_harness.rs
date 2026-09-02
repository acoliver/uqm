//! P00 Symbol Harness — Production Member Extraction Proof
//!
//! This binary proves that the deterministic C archive `libuqm_c.a` contains
//! extractable definitions of the seven production symbols the execution
//! contract requires (`@plan PLAN-20260723-RUNTIME-AUTOMATION.P00` §8):
//!
//!   DoInput, AnyButtonPress            (gameinp_rust_main.o)
//!   DoConfirmExit                      (confirm.c.o)
//!   TFB_ProcessEvents, TFB_SwapBuffers (sdl_common.c.o)
//!   ProcessInputEvent                  (input.c.o)
//!   TFB_FlushGraphicsEx                (dcqueue.c.o)
//!
//! Each symbol is declared extern and its address is stored in a `#[used]`
//! static table, so the linker can neither discard the references nor link
//! the binary without extracting the corresponding archive members. This
//! replaces the last first-party C harness (`harness/p00_harness.c`), which
//! used the same mechanism through a C translation unit.
//!
//! The declarations are intentionally type-erased to `unsafe extern "C" fn`:
//! they exist solely to create symbol references for the linker, are never
//! called, and uniform signatures are the only form a single homogeneous
//! pointer table admits without `unsafe` pointer conversion. The link itself
//! (and the retained `nm` evidence over this binary) is the proof.
//!
//! Usage: cargo run --features linked_c_archive --bin p00_symbol_harness
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P00 §8

extern "C" {
    fn DoInput();
    fn AnyButtonPress();
    fn DoConfirmExit();
    fn TFB_ProcessEvents();
    fn TFB_SwapBuffers();
    fn ProcessInputEvent();
    fn TFB_FlushGraphicsEx();
}

/// Address-of table for the seven required production symbols.
///
/// `#[used]` keeps the table in the final binary; its relocations force the
/// linker to extract each referenced member from `libuqm_c.a`, leaving
/// unreferenced members out.
#[used]
static PRODUCTION_SYMBOLS: [unsafe extern "C" fn(); 7] = [
    DoInput,
    AnyButtonPress,
    DoConfirmExit,
    TFB_ProcessEvents,
    TFB_SwapBuffers,
    ProcessInputEvent,
    TFB_FlushGraphicsEx,
];

fn main() {
    // Reaching this line already proves the link resolved every table entry.
    println!("harness_symbol_count={}", PRODUCTION_SYMBOLS.len());
    println!("RESULT=PASS");
}
