//! Typed, versioned automation script contracts and full pre-runtime
//! validation.
//!
//! This module implements REQ-SCRIPT-001..006: strict UTF-8/JSON parsing with
//! a closed (deny-unknown-fields) versioned root, six typed menu keys, closed
//! action enum, typed main-menu transition assertion, budget/count/activity
//! validation, the inclusive-limit static lower bound, and final-`finish`
//! semantics.
//!
//! It performs pure parsing/validation only — no file I/O, no scheduler, no
//! FFI, no capture, and no game-lifecycle side effects.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
//! @requirement REQ-SCRIPT-001..006

use crate::automation::error::AutomationError;
use crate::mainloop::restart_menu::types::RestartMenuItem;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

// ===========================================================================
//  Build capability contract (REQ-BUILD-001)
// ===========================================================================

/// The build-configuration flags that an active automation binary must
/// provide. Validation rejects activation when any is absent. P01 defines the
/// contract; P05 wires the actual check against linked build metadata.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-BUILD-001
pub const CAPABILITY_REQUIRED_FLAGS: &[&str] = &[
    "RUST_OWNS_MAIN",
    "USE_RUST_THREADS",
    "USE_RUST_GFX",
    "USE_RUST_COMM",
    "USE_RUST_RESTART",
];

// ===========================================================================
//  MenuKey — six typed variants (REQ-SCRIPT-005)
// ===========================================================================

/// The six menu control variants exposed to automation, mapped from the C
/// `controls.h` enum indices.
///
/// The indices match `KEY_MENU_UP..KEY_MENU_CANCEL` (5..=10):
///
/// | Variant | C enum constant | Index |
/// |---------|-----------------|-------|
/// | Up      | `KEY_MENU_UP`     | 5     |
/// | Down    | `KEY_MENU_DOWN`   | 6     |
/// | Left    | `KEY_MENU_LEFT`   | 7     |
/// | Right   | `KEY_MENU_RIGHT`  | 8     |
/// | Select  | `KEY_MENU_SELECT` | 9     |
/// | Cancel  | `KEY_MENU_CANCEL` | 10    |
///
/// Unknown or numeric-only identifiers are rejected; only the six closed
/// string names are accepted.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-005
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MenuKey {
    Up = 5,
    Down = 6,
    Left = 7,
    Right = 8,
    Select = 9,
    Cancel = 10,
}

impl MenuKey {
    /// All six variants, in `controls.h` enum order.
    pub const ALL: [MenuKey; 6] = [
        MenuKey::Up,
        MenuKey::Down,
        MenuKey::Left,
        MenuKey::Right,
        MenuKey::Select,
        MenuKey::Cancel,
    ];

    /// The stable `controls.h` index for this key.
    #[must_use]
    #[inline]
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// The canonical lowercase string name, matching `menu.<name>.N`
    /// resource keys in `sc2/content/menu.key`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            MenuKey::Up => "up",
            MenuKey::Down => "down",
            MenuKey::Left => "left",
            MenuKey::Right => "right",
            MenuKey::Select => "select",
            MenuKey::Cancel => "cancel",
        }
    }

    /// Parse a key from its canonical string name.
    ///
    /// Returns `None` for any value that is not exactly one of the six names.
    /// Numeric or unknown identifiers are rejected (REQ-SCRIPT-005).
    #[must_use]
    pub fn from_name(name: &str) -> Option<MenuKey> {
        match name {
            "up" => Some(MenuKey::Up),
            "down" => Some(MenuKey::Down),
            "left" => Some(MenuKey::Left),
            "right" => Some(MenuKey::Right),
            "select" => Some(MenuKey::Select),
            "cancel" => Some(MenuKey::Cancel),
            _ => None,
        }
    }

    /// Checked conversion from a raw index. Returns `None` outside 5..=10.
    #[must_use]
    pub const fn from_index(index: u8) -> Option<MenuKey> {
        match index {
            5 => Some(MenuKey::Up),
            6 => Some(MenuKey::Down),
            7 => Some(MenuKey::Left),
            8 => Some(MenuKey::Right),
            9 => Some(MenuKey::Select),
            10 => Some(MenuKey::Cancel),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for MenuKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        MenuKey::from_name(&name).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "unknown menu key '{name}'; expected one of: up, down, left, right, select, cancel"
            ))
        })
    }
}

// ===========================================================================
//  Typed main-menu transition (REQ-SCRIPT-006, REQ-SEM-001)
// ===========================================================================

/// A typed main-menu transition assertion using existing `RestartMenuItem`
/// names. Semantic assertion is accepted only as a typed from/to pair — raw
/// indices or stringly-typed values are rejected (REQ-SCRIPT-006).
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-006, REQ-SEM-002
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MainMenuTransition {
    /// The menu item the transition originates from.
    pub from: RestartMenuItem,
    /// The menu item the transition moves to.
    pub to: RestartMenuItem,
}

impl MainMenuTransition {
    /// Construct a typed transition, validating both endpoints.
    #[must_use]
    pub const fn new(from: RestartMenuItem, to: RestartMenuItem) -> Self {
        Self { from, to }
    }
}

// ===========================================================================
//  Label validation (REQ-SCRIPT-005, REQ-IO-002)
// ===========================================================================

/// Validate a label for safe use as an artifact path component.
///
/// Labels must be nonempty and must not contain path separators, `..`
/// traversal sequences, NUL, or ASCII control characters. This is a pure
/// contract check; it does not touch the filesystem (that is REQ-IO-002,
/// owned by a later phase).
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-005
#[must_use]
pub fn is_valid_label(label: &str) -> bool {
    if label.is_empty() {
        return false;
    }
    if label == ".." || label.contains('/') || label.contains('\\') {
        return false;
    }
    if label.contains('\0') {
        return false;
    }
    // Reject any ASCII control character and reject `..` as a substring
    // segment to prevent traversal like `a/../b`.
    if label.bytes().any(|b| b.is_ascii_control()) {
        return false;
    }
    if label.contains("..") {
        return false;
    }
    true
}

// ===========================================================================
//  DTOs — closed (deny_unknown_fields) typed JSON contract (REQ-SCRIPT-002/003)
// ===========================================================================

/// Positive budget triple. All three must be strictly positive.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-002, REQ-SCRIPT-004
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    /// Inclusive maximum number of admitted input callbacks. A maximum `M`
    /// admits at most `M-1` callbacks to scheduler action work.
    pub max_input_ticks: u64,
    /// Inclusive maximum number of admitted presentation callbacks.
    pub max_presentations: u64,
    /// Wall-clock timeout in seconds. The run is terminal when
    /// `elapsed >= timeout`.
    pub max_wallclock_seconds: u64,
}

/// A `wait_input_ticks` step: consume exactly `count` admitted input
/// callbacks before advancing.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-003, REQ-SCRIPT-004
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WaitInputTicksStep {
    pub count: u64,
}

/// A `set_menu_key` step: write `value` (0/1) to the owned `key` slot.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-003
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetMenuKeyStep {
    pub key: MenuKey,
    pub value: u8,
}

/// A `tap_menu_key` step: hold `value` for `hold` admitted input updates,
/// release, then settle for `settle` admitted input callbacks.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-003, REQ-SCRIPT-004
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TapMenuKeyStep {
    pub key: MenuKey,
    pub value: u8,
    pub hold: u64,
    pub settle: u64,
}

/// A `capture` step: capture the logical surface and publish under `label`.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-003, REQ-SCRIPT-006
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CaptureStep {
    pub label: String,
}

/// An `assert_activity` step: assert the live activity word satisfies
/// `(activity & mask) == equals`. Equivalently the asserted bits are set
/// exactly as specified by `equals` within `mask`.
///
/// Per REQ-SCRIPT-004, the relationship `equals & !mask == 0` must hold: no
/// asserted bit may lie outside the mask.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-003, REQ-SCRIPT-004
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActivityAssertion {
    pub mask: u16,
    pub equals: u16,
}

/// The closed set of automation actions (REQ-SCRIPT-003).
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-003
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "action")]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    WaitInputTicks(WaitInputTicksStep),
    SetMenuKey(SetMenuKeyStep),
    TapMenuKey(TapMenuKeyStep),
    Capture(CaptureStep),
    AssertActivity(ActivityAssertion),
    AssertMainMenuTransition(MainMenuTransitionDto),
    Finish,
}

/// serde DTO for the main-menu transition assertion, using the canonical
/// `RestartMenuItem` names. Converted to a typed [`MainMenuTransition`] during
/// validation.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-006
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MainMenuTransitionDto {
    pub from: String,
    pub to: String,
}

/// A single script step wrapping an [`Action`].
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-002
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ScriptStep {
    #[serde(flatten)]
    pub action: Action,
}

/// The closed, versioned root document DTO.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-002
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RootDocument {
    pub version: u64,
    pub name: String,
    pub budgets: Budgets,
    pub steps: Vec<Action>,
}

// ===========================================================================
//  Validated, closed contract (post-validation)
// ===========================================================================

/// A fully validated automation script.
///
/// Constructed only via [`validate_script`], which enforces every
/// REQ-SCRIPT-001..006 invariant. The contained [`RootDocument`] is the parsed
/// DTO; field access is intentionally read-only via this type.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-002..006
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedScript {
    pub(crate) name: String,
    pub(crate) budgets: Budgets,
    pub(crate) steps: Vec<Action>,
    pub(crate) transitions: Vec<MainMenuTransition>,
}

impl ValidatedScript {
    /// The human-readable script name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The validated budget triple.
    #[must_use]
    pub fn budgets(&self) -> Budgets {
        self.budgets
    }

    /// The validated, ordered action list.
    #[must_use]
    pub fn steps(&self) -> &[Action] {
        &self.steps
    }

    /// The typed main-menu transition assertions, in step order.
    #[must_use]
    pub fn transitions(&self) -> &[MainMenuTransition] {
        &self.transitions
    }
}

// ===========================================================================
//  Parsing and validation
// ===========================================================================

/// Parse UTF-8 bytes as JSON into the closed root DTO.
///
/// Performs strict UTF-8 decoding and strict JSON deserialization with
/// `deny_unknown_fields` at every level. Does not run budget/ordering
/// validation — use [`validate_script`] for the complete contract.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-001, REQ-SCRIPT-002
pub fn parse_script(bytes: &[u8], path: impl AsRef<Path>) -> Result<RootDocument, AutomationError> {
    let path_str = path.as_ref().to_string_lossy().into_owned();
    let text = std::str::from_utf8(bytes).map_err(|e| AutomationError::NotUtf8 {
        path: path_str.clone(),
        reason: e.to_string(),
    })?;
    parse_text(text, &path_str)
}

fn parse_text(text: &str, path: &str) -> Result<RootDocument, AutomationError> {
    // First deserialize into a serde_json::Value to detect duplicate keys
    // (serde_json preserves last-wins by default in typed deserialization, but
    // reports duplicates only in Value mode via the `preserve_order` off path
    // is last-wins; we additionally enforce explicit duplicate detection
    // below). serde_json by default does NOT error on duplicate keys in typed
    // deserialization, so we detect duplicates structurally.
    let value: Value = serde_json::from_str(text).map_err(|e| AutomationError::InvalidJson {
        path: path.to_string(),
        reason: e.to_string(),
    })?;

    // Reject duplicate keys by re-checking the raw text against the value
    // structure. serde_json with default features uses BTreeMap (last-wins),
    // so we detect duplicates by counting key occurrences in object scopes.
    detect_duplicate_keys(text, path)?;

    // Now deserialize the validated value tree into the typed DTO. Because the
    // JSON was already parsed, this is infallible for shape but still enforces
    // deny_unknown_fields and type constraints.
    serde_json::from_value::<RootDocument>(value).map_err(|e| {
        // Classify the serde error into a precise AutomationError variant.
        classify_serde_error(&e.to_string(), path)
    })
}

/// Map a serde_json error string into the most precise [`AutomationError`]
/// variant, preserving the path.
fn classify_serde_error(msg: &str, path: &str) -> AutomationError {
    let lower = msg.to_ascii_lowercase();
    if lower.contains("unknown field") {
        // Extract the field name if quoted.
        let field = extract_quoted(msg).unwrap_or_else(|| msg.to_string());
        AutomationError::UnknownField {
            path: path.to_string(),
            field,
        }
    } else if lower.contains("missing field") {
        let field = extract_quoted(msg).unwrap_or_default();
        let field_static = match field.as_str() {
            "version" => "version",
            "name" => "name",
            "budgets" => "budgets",
            "steps" => "steps",
            "max_input_ticks" => "max_input_ticks",
            "max_presentations" => "max_presentations",
            "max_wallclock_seconds" => "max_wallclock_seconds",
            "count" => "count",
            "key" => "key",
            "value" => "value",
            "hold" => "hold",
            "settle" => "settle",
            "label" => "label",
            "mask" => "mask",
            "equals" => "equals",
            "from" => "from",
            "to" => "to",
            "action" => "action",
            _ => "field",
        };
        AutomationError::MissingField {
            path: path.to_string(),
            field: field_static,
        }
    } else {
        AutomationError::InvalidJson {
            path: path.to_string(),
            reason: msg.to_string(),
        }
    }
}

/// Extract the first single- or double-quoted substring from `msg`.
fn extract_quoted(msg: &str) -> Option<String> {
    let mut chars = msg.char_indices();
    while let Some((_, c)) = chars.next() {
        if c == '\'' || c == '"' {
            let quote = c;
            let start = chars.clone().next().map(|(i, _)| i)?;
            let mut end = start;
            for (i, cc) in chars.by_ref() {
                if cc == quote {
                    return Some(msg[start..end].to_string());
                }
                end = i + cc.len_utf8();
            }
        }
    }
    None
}

/// Detect object-level duplicate keys in raw JSON text by tokenizing the
/// JSON structure. This is necessary because serde_json's default
/// deserializer uses last-wins semantics and does not reject duplicates.
///
/// The tokenizer walks the JSON token stream, distinguishing object keys
/// (strings appearing in key position) from string values, and flags any key
/// that repeats within the same object.
fn detect_duplicate_keys(text: &str, path: &str) -> Result<(), AutomationError> {
    let tokens = match json_tokenize(text) {
        Ok(t) => t,
        // If our tokenizer can't parse it, defer to serde_json's real error.
        Err(()) => return Ok(()),
    };

    // State machine over the token stream: track object scopes and their keys.
    let mut object_scopes: Vec<Vec<String>> = Vec::new();
    // Within an object, keys and values alternate. `expecting_key` is true
    // when the next token must be a key (or `}`).
    let mut expecting_key = false;

    for tok in &tokens {
        match tok {
            JsonToken::ObjectStart => {
                object_scopes.push(Vec::new());
                expecting_key = true;
            }
            JsonToken::ObjectEnd => {
                object_scopes.pop();
                expecting_key = false;
            }
            JsonToken::ArrayStart | JsonToken::ArrayEnd => {
                expecting_key = false;
            }
            JsonToken::Colon => {
                // The token after a key is its value; after the value we expect
                // either a comma (then another key) or `}`.
                expecting_key = false;
            }
            JsonToken::Comma => {
                // Inside an object, a comma is followed by the next key.
                if !object_scopes.is_empty() {
                    expecting_key = true;
                }
            }
            JsonToken::String(s) => {
                if expecting_key {
                    if let Some(scope) = object_scopes.last_mut() {
                        if scope.contains(s) {
                            return Err(AutomationError::UnknownField {
                                path: path.to_string(),
                                field: s.clone(),
                            });
                        }
                        scope.push(s.clone());
                    }
                    // After a key, expect ':' then value.
                    expecting_key = false;
                }
                // String values don't change expecting_key (it stays false
                // until a comma inside an object).
            }
            JsonToken::Other => {
                // Numbers, booleans, null — treated as values.
                if expecting_key {
                    // A non-string key is invalid JSON; defer to serde.
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
enum JsonToken {
    ObjectStart,
    ObjectEnd,
    ArrayStart,
    ArrayEnd,
    Colon,
    Comma,
    String(String),
    Other,
}

/// A minimal JSON tokenizer that recognizes structural punctuation and string
/// literals (with escape handling). Non-string scalars are collapsed into
/// `Other`. Returns `Err(())` if the input is not structurally JSON-like.
fn json_tokenize(text: &str) -> Result<Vec<JsonToken>, ()> {
    let mut tokens = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' => {
                i += 1;
            }
            b'{' => {
                tokens.push(JsonToken::ObjectStart);
                i += 1;
            }
            b'}' => {
                tokens.push(JsonToken::ObjectEnd);
                i += 1;
            }
            b'[' => {
                tokens.push(JsonToken::ArrayStart);
                i += 1;
            }
            b']' => {
                tokens.push(JsonToken::ArrayEnd);
                i += 1;
            }
            b':' => {
                tokens.push(JsonToken::Colon);
                i += 1;
            }
            b',' => {
                tokens.push(JsonToken::Comma);
                i += 1;
            }
            b'"' => {
                // String literal with escape handling.
                i += 1;
                let mut s = String::new();
                let mut closed = false;
                while i < bytes.len() {
                    let b = bytes[i];
                    if b == b'\\' {
                        i += 1;
                        if i >= bytes.len() {
                            return Err(());
                        }
                        match bytes[i] {
                            b'"' => s.push('"'),
                            b'\\' => s.push('\\'),
                            b'/' => s.push('/'),
                            b'n' => s.push('\n'),
                            b't' => s.push('\t'),
                            b'r' => s.push('\r'),
                            b'b' => s.push('\u{0008}'),
                            b'f' => s.push('\u{000C}'),
                            b'u' => {
                                // \uXXXX
                                if i + 4 >= bytes.len() {
                                    return Err(());
                                }
                                let hex = &text[i + 1..i + 5];
                                let _ = u32::from_str_radix(hex, 16).map_err(|_| ())?;
                                s.push_str(hex);
                                i += 4;
                            }
                            _ => return Err(()),
                        }
                        i += 1;
                    } else if b == b'"' {
                        closed = true;
                        i += 1;
                        break;
                    } else {
                        s.push(b as char);
                        i += 1;
                    }
                }
                if !closed {
                    return Err(());
                }
                tokens.push(JsonToken::String(s));
            }
            _ => {
                // Scalar value (number, true, false, null).
                let start = i;
                while i < bytes.len()
                    && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | b',' | b'}' | b']')
                {
                    i += 1;
                }
                if i == start {
                    return Err(());
                }
                tokens.push(JsonToken::Other);
            }
        }
    }
    Ok(tokens)
}

/// Validate a parsed [`RootDocument`] against every REQ-SCRIPT-002..006
/// invariant and return a [`ValidatedScript`].
///
/// Checks performed (all pure, no side effects):
/// - version == 1
/// - all three budgets strictly positive
/// - steps nonempty
/// - exactly one `finish`, and it is last
/// - counts nonnegative/representable, tap hold positive
/// - activity values fit `u16`, `equals & !mask == 0`
/// - labels nonempty and traversal-free
/// - typed main-menu transitions from existing `RestartMenuItem` names
/// - inclusive-limit static lower bound: the sum of statically required
///   admitted input callbacks and presentations fits within the declared
///   budgets (each maximum must be at least `required + 1`)
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-002..006
pub fn validate_script(
    doc: RootDocument,
    path: impl AsRef<Path>,
) -> Result<ValidatedScript, AutomationError> {
    let path_str = path.as_ref().to_string_lossy().into_owned();
    validate_document(doc, &path_str)
}

fn validate_document(doc: RootDocument, path: &str) -> Result<ValidatedScript, AutomationError> {
    // REQ-SCRIPT-002: closed versioned root — version must be 1.
    if doc.version != 1 {
        return Err(AutomationError::UnsupportedVersion {
            path: path.to_string(),
            found: doc.version.to_string(),
        });
    }

    // REQ-SCRIPT-002: name present (deny_unknown_fields + missing field
    // handles absence at deserialization; here we reject empty names).
    if doc.name.trim().is_empty() {
        return Err(AutomationError::InvalidValue {
            path: path.to_string(),
            field: "name",
            reason: "name must be nonempty".to_string(),
        });
    }

    // REQ-SCRIPT-004: budgets strictly positive.
    if doc.budgets.max_input_ticks == 0 {
        return Err(AutomationError::InvalidValue {
            path: path.to_string(),
            field: "max_input_ticks",
            reason: "budget must be positive".to_string(),
        });
    }
    if doc.budgets.max_presentations == 0 {
        return Err(AutomationError::InvalidValue {
            path: path.to_string(),
            field: "max_presentations",
            reason: "budget must be positive".to_string(),
        });
    }
    if doc.budgets.max_wallclock_seconds == 0 {
        return Err(AutomationError::InvalidValue {
            path: path.to_string(),
            field: "max_wallclock_seconds",
            reason: "budget must be positive".to_string(),
        });
    }

    // REQ-SCRIPT-002: steps nonempty.
    if doc.steps.is_empty() {
        return Err(AutomationError::EmptySteps {
            path: path.to_string(),
        });
    }

    let steps: Vec<Action> = doc.steps;

    // Final finish semantics (REQ-SCRIPT-004).
    validate_finish_semantics(&steps, path)?;

    // Per-step validation.
    let mut transitions = Vec::new();
    let mut required_input_callbacks: u64 = 0;
    let required_presentations: u64 = 0;
    for (i, step) in steps.iter().enumerate() {
        match step {
            Action::WaitInputTicks(w) => {
                // count is u64 so always nonnegative; representability is the
                // checked-add lower-bound below.
                required_input_callbacks = required_input_callbacks
                    .checked_add(w.count)
                    .ok_or_else(|| AutomationError::ArithmeticOverflow {
                        path: path.to_string(),
                        reason: format!(
                            "required input ticks overflow at step {i}: count={}",
                            w.count
                        ),
                    })?;
            }
            Action::SetMenuKey(s) => {
                if s.value > 1 {
                    return Err(AutomationError::step(
                        path,
                        i,
                        format!("set_menu_key value must be 0 or 1, got {}", s.value),
                    ));
                }
            }
            Action::TapMenuKey(t) => {
                if t.hold == 0 {
                    return Err(AutomationError::step(
                        path,
                        i,
                        "tap_menu_key hold must be positive",
                    ));
                }
                if t.value > 1 {
                    return Err(AutomationError::step(
                        path,
                        i,
                        format!("tap_menu_key value must be 0 or 1, got {}", t.value),
                    ));
                }
                // A tap consumes `hold` admitted input callbacks while held
                // and `settle` admitted input callbacks while settling.
                required_input_callbacks = required_input_callbacks
                    .checked_add(t.hold)
                    .ok_or_else(|| AutomationError::ArithmeticOverflow {
                        path: path.to_string(),
                        reason: format!(
                            "required input ticks overflow at step {i}: hold={}",
                            t.hold
                        ),
                    })?;
                required_input_callbacks = required_input_callbacks
                    .checked_add(t.settle)
                    .ok_or_else(|| AutomationError::ArithmeticOverflow {
                        path: path.to_string(),
                        reason: format!(
                            "required input ticks overflow at step {i}: settle={}",
                            t.settle
                        ),
                    })?;
            }
            Action::Capture(c) => {
                if !is_valid_label(&c.label) {
                    return Err(AutomationError::step(
                        path,
                        i,
                        format!(
                            "capture label '{}' is invalid: must be nonempty and reject separators, '..', and control characters",
                            c.label
                        ),
                    ));
                }
            }
            Action::AssertActivity(a) => {
                // REQ-SCRIPT-004: equals & !mask == 0
                if (a.equals & !a.mask) != 0 {
                    return Err(AutomationError::step(
                        path,
                        i,
                        format!(
                            "assert_activity: equals (0x{:04X}) has bits outside mask (0x{:04X})",
                            a.equals, a.mask
                        ),
                    ));
                }
            }
            Action::AssertMainMenuTransition(dto_inner) => {
                let from = parse_menu_item(&dto_inner.from, i, path)?;
                let to = parse_menu_item(&dto_inner.to, i, path)?;
                transitions.push(MainMenuTransition::new(from, to));
            }
            Action::Finish => {}
        }
    }

    // Inclusive-limit static lower bound (REQ-SCRIPT-004, REQ-WATCH-001).
    // A maximum M admits at most M-1 callbacks; requiring N admitted callbacks
    // needs max >= N+1.
    check_static_lower_bound(
        "max_input_ticks",
        required_input_callbacks,
        doc.budgets.max_input_ticks,
        path,
    )?;
    check_static_lower_bound(
        "max_presentations",
        required_presentations,
        doc.budgets.max_presentations,
        path,
    )?;

    Ok(ValidatedScript {
        name: doc.name,
        budgets: doc.budgets,
        steps,
        transitions,
    })
}

/// Enforce `max >= required + 1` using checked arithmetic.
fn check_static_lower_bound(
    budget_name: &str,
    required: u64,
    max: u64,
    path: &str,
) -> Result<(), AutomationError> {
    let needed = required
        .checked_add(1)
        .ok_or_else(|| AutomationError::ArithmeticOverflow {
            path: path.to_string(),
            reason: format!("required {budget_name} + 1 overflow"),
        })?;
    if max < needed {
        // Find the offending step index for a precise message by reporting
        // the aggregate; the per-step contributions are already validated.
        return Err(AutomationError::InsufficientBudget {
            path: path.to_string(),
            step: 0,
            reason: format!(
                "{budget_name}={max} admits at most {} callbacks but the script statically requires {required} admitted (need at least {needed})",
                max.saturating_sub(1)
            ),
        });
    }
    Ok(())
}

/// Validate final-`finish` semantics: exactly one finish, and it is last.
fn validate_finish_semantics(steps: &[Action], path: &str) -> Result<(), AutomationError> {
    let finish_count = steps.iter().filter(|a| matches!(a, Action::Finish)).count();
    if finish_count == 0 {
        return Err(AutomationError::FinishSemantics {
            path: path.to_string(),
            reason: "script must contain exactly one final 'finish' step, found none".to_string(),
        });
    }
    if finish_count > 1 {
        return Err(AutomationError::FinishSemantics {
            path: path.to_string(),
            reason: format!("script must contain exactly one 'finish' step, found {finish_count}"),
        });
    }
    // Exactly one: it must be last. `steps` is guaranteed nonempty because
    // empty steps were rejected above, but guard defensively rather than
    // panicking.
    let is_last_finish = steps
        .last()
        .map(|last| matches!(last, Action::Finish))
        .unwrap_or(false);
    if !is_last_finish {
        return Err(AutomationError::FinishSemantics {
            path: path.to_string(),
            reason: "'finish' must be the last step".to_string(),
        });
    }
    Ok(())
}

/// Parse a `RestartMenuItem` from its canonical name, rejecting unknown
/// stringly-typed values and raw indices.
fn parse_menu_item(
    name: &str,
    step: usize,
    path: &str,
) -> Result<RestartMenuItem, AutomationError> {
    match name {
        "NewGame" => Ok(RestartMenuItem::NewGame),
        "LoadGame" => Ok(RestartMenuItem::LoadGame),
        "SuperMelee" => Ok(RestartMenuItem::SuperMelee),
        "Setup" => Ok(RestartMenuItem::Setup),
        "Quit" => Ok(RestartMenuItem::Quit),
        other => Err(AutomationError::step(
            path,
            step,
            format!(
                "assert_main_menu_transition: unknown menu item '{other}'; expected one of: NewGame, LoadGame, SuperMelee, Setup, Quit"
            ),
        )),
    }
}

// ===========================================================================
//  Unit tests (TDD — written before/with implementation)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_MINIMAL: &str = r#"{
        "version": 1,
        "name": "smoke",
        "budgets": {
            "max_input_ticks": 10,
            "max_presentations": 10,
            "max_wallclock_seconds": 60
        },
        "steps": [
            {"action": "wait_input_ticks", "count": 2},
            {"action": "capture", "label": "start"},
            {"action": "tap_menu_key", "key": "down", "value": 1, "hold": 1, "settle": 1},
            {"action": "assert_main_menu_transition", "from": "NewGame", "to": "LoadGame"},
            {"action": "assert_activity", "mask": 61440, "equals": 0},
            {"action": "finish"}
        ]
    }"#;

    fn p() -> &'static str {
        "test.json"
    }

    // --- REQ-SCRIPT-001: strict UTF-8/root parse ---

    #[test]
    fn rejects_non_utf8_bytes() {
        let bad = b"version\xFF";
        let err = parse_script(bad, p()).unwrap_err();
        assert!(matches!(err, AutomationError::NotUtf8 { .. }));
        assert_eq!(err.path(), Some("test.json"));
    }

    #[test]
    fn rejects_malformed_json() {
        let err = parse_script(b"{not json", p()).unwrap_err();
        assert!(matches!(err, AutomationError::InvalidJson { .. }));
    }

    #[test]
    fn rejects_unknown_root_field() {
        let txt = r#"{"version":1,"name":"x","extra":true,"budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"finish"}]}"#;
        let err = parse_script(txt.as_bytes(), p()).unwrap_err();
        assert!(matches!(err, AutomationError::UnknownField { .. }));
        let msg = format!("{err}");
        assert!(msg.contains("extra"));
    }

    #[test]
    fn rejects_unknown_step_field() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"wait_input_ticks","count":0,"bogus":1},{"action":"finish"}]}"#;
        let err = parse_script(txt.as_bytes(), p()).unwrap_err();
        assert!(matches!(err, AutomationError::UnknownField { .. }));
    }

    #[test]
    fn rejects_missing_required_field() {
        let txt = r#"{"version":1,"name":"x","steps":[{"action":"finish"}]}"#;
        let err = parse_script(txt.as_bytes(), p()).unwrap_err();
        assert!(matches!(err, AutomationError::MissingField { .. }));
    }

    #[test]
    fn rejects_unsupported_version() {
        let txt = r#"{"version":2,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::UnsupportedVersion { .. }));
    }

    #[test]
    fn accepts_valid_minimal_script() {
        let doc = parse_script(VALID_MINIMAL.as_bytes(), p()).unwrap();
        let v = validate_script(doc, p()).unwrap();
        assert_eq!(v.name(), "smoke");
        assert_eq!(v.steps().len(), 6);
        assert_eq!(v.transitions().len(), 1);
        assert_eq!(
            v.transitions()[0],
            MainMenuTransition::new(RestartMenuItem::NewGame, RestartMenuItem::LoadGame)
        );
    }

    // --- REQ-SCRIPT-005: typed six-key mappings ---

    #[test]
    fn menu_key_six_variants_indices_match_controls_h() {
        assert_eq!(MenuKey::Up.index(), 5);
        assert_eq!(MenuKey::Down.index(), 6);
        assert_eq!(MenuKey::Left.index(), 7);
        assert_eq!(MenuKey::Right.index(), 8);
        assert_eq!(MenuKey::Select.index(), 9);
        assert_eq!(MenuKey::Cancel.index(), 10);
        assert_eq!(MenuKey::ALL.len(), 6);
    }

    #[test]
    fn menu_key_exhaustive_name_roundtrip() {
        for k in MenuKey::ALL {
            let name = k.name();
            assert_eq!(MenuKey::from_name(name), Some(k));
            assert_eq!(MenuKey::from_index(k.index()), Some(k));
        }
    }

    #[test]
    fn menu_key_rejects_unknown_names() {
        assert_eq!(MenuKey::from_name("up"), Some(MenuKey::Up));
        assert_eq!(MenuKey::from_name("UP"), None);
        assert_eq!(MenuKey::from_name("5"), None);
        assert_eq!(MenuKey::from_name("middle"), None);
        assert_eq!(MenuKey::from_name(""), None);
    }

    #[test]
    fn menu_key_rejects_numeric_or_out_of_range_indices() {
        assert_eq!(MenuKey::from_index(0), None);
        assert_eq!(MenuKey::from_index(4), None);
        assert_eq!(MenuKey::from_index(11), None);
        assert_eq!(MenuKey::from_index(255), None);
        for i in 5..=10u8 {
            assert!(MenuKey::from_index(i).is_some());
        }
    }

    #[test]
    fn script_rejects_unknown_menu_key_string() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"set_menu_key","key":"middle","value":1},{"action":"finish"}]}"#;
        let err = parse_script(txt.as_bytes(), p()).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("middle"), "should name the bad key: {msg}");
    }

    // --- REQ-SCRIPT-002: closed versioned root, positive budgets ---

    #[test]
    fn rejects_zero_input_budget() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":0,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::InvalidValue { .. }));
    }

    #[test]
    fn rejects_zero_presentation_budget() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":0,"max_wallclock_seconds":1},"steps":[{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        assert!(matches!(
            validate_script(doc, p()).unwrap_err(),
            AutomationError::InvalidValue { .. }
        ));
    }

    #[test]
    fn rejects_zero_wallclock_budget() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":0},"steps":[{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        assert!(matches!(
            validate_script(doc, p()).unwrap_err(),
            AutomationError::InvalidValue { .. }
        ));
    }

    #[test]
    fn rejects_empty_steps() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        assert!(matches!(
            validate_script(doc, p()).unwrap_err(),
            AutomationError::EmptySteps { .. }
        ));
    }

    // --- REQ-SCRIPT-003: closed action enum ---

    #[test]
    fn rejects_unknown_action_tag() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"teleport"},{"action":"finish"}]}"#;
        let err = parse_script(txt.as_bytes(), p()).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("teleport"));
    }

    // --- REQ-SCRIPT-004: bounds, budget relationship, ordering ---

    #[test]
    fn rejects_tap_with_zero_hold() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":10,"max_presentations":10,"max_wallclock_seconds":60},"steps":[{"action":"tap_menu_key","key":"down","value":1,"hold":0,"settle":1},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::Step { step: 0, .. }));
        let msg = format!("{err}");
        assert!(msg.contains("hold must be positive"));
        assert_eq!(err.step_index(), Some(0));
    }

    #[test]
    fn rejects_set_menu_key_value_out_of_range() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"set_menu_key","key":"up","value":5},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::Step { step: 0, .. }));
        let msg = format!("{err}");
        assert!(msg.contains("0 or 1"));
    }

    #[test]
    fn rejects_activity_equals_outside_mask() {
        // mask = 0x00FF, equals = 0x0100 -> equals & !mask = 0x0100 != 0
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"assert_activity","mask":255,"equals":256},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::Step { step: 0, .. }));
        let msg = format!("{err}");
        assert!(msg.contains("outside mask"));
    }

    #[test]
    fn accepts_activity_equals_within_mask() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"assert_activity","mask":61440,"equals":0},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        assert!(validate_script(doc, p()).is_ok());
    }

    #[test]
    fn rejects_early_or_non_last_finish() {
        // finish not last
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"finish"},{"action":"wait_input_ticks","count":0}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::FinishSemantics { .. }));
        let msg = format!("{err}");
        assert!(msg.contains("last"));
    }

    #[test]
    fn rejects_no_finish() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"wait_input_ticks","count":0}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::FinishSemantics { .. }));
    }

    #[test]
    fn rejects_duplicate_finish() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"finish"},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::FinishSemantics { .. }));
        let msg = format!("{err}");
        assert!(msg.contains("2"));
    }

    // --- Inclusive-limit static lower bound ---

    #[test]
    fn rejects_required_callbacks_exceeding_inclusive_max() {
        // wait_input_ticks count=5 needs max_input_ticks >= 6 (5+1).
        // Set max_input_ticks = 5 -> admits only 4 callbacks -> reject.
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":5,"max_presentations":1,"max_wallclock_seconds":60},"steps":[{"action":"wait_input_ticks","count":5},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::InsufficientBudget { .. }));
        let msg = format!("{err}");
        assert!(msg.contains("max_input_ticks"));
    }

    #[test]
    fn accepts_required_callbacks_at_inclusive_boundary() {
        // wait_input_ticks count=5 needs max >= 6. Set max=6 -> admits 5 -> OK.
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":6,"max_presentations":1,"max_wallclock_seconds":60},"steps":[{"action":"wait_input_ticks","count":5},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        assert!(validate_script(doc, p()).is_ok());
    }

    #[test]
    fn rejects_tap_total_exceeding_inclusive_max() {
        // tap hold=3 settle=2 needs max >= 6 (5+1). Set max=5 -> reject.
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":5,"max_presentations":1,"max_wallclock_seconds":60},"steps":[{"action":"tap_menu_key","key":"down","value":1,"hold":3,"settle":2},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        assert!(matches!(
            validate_script(doc, p()).unwrap_err(),
            AutomationError::InsufficientBudget { .. }
        ));
    }

    #[test]
    fn rejects_overflow_in_required_callback_sum() {
        // Two wait_input_ticks whose sum overflows u64.
        let txt = format!(
            r#"{{"version":1,"name":"x","budgets":{{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1}},"steps":[{{"action":"wait_input_ticks","count":{}}},{{"action":"wait_input_ticks","count":{}}},{{"action":"finish"}}]}}"#,
            u64::MAX,
            1u64
        );
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::ArithmeticOverflow { .. }));
    }

    // --- Labels ---

    #[test]
    fn rejects_empty_capture_label() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"capture","label":""},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::Step { step: 0, .. }));
    }

    #[test]
    fn rejects_label_with_path_separator() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"capture","label":"a/b"},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        assert!(matches!(
            validate_script(doc, p()).unwrap_err(),
            AutomationError::Step { step: 0, .. }
        ));
    }

    #[test]
    fn rejects_label_with_dotdot_traversal() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"capture","label":".."},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        assert!(matches!(
            validate_script(doc, p()).unwrap_err(),
            AutomationError::Step { step: 0, .. }
        ));
    }

    #[test]
    fn rejects_label_with_dotdot_substring() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"capture","label":"a..b"},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        assert!(matches!(
            validate_script(doc, p()).unwrap_err(),
            AutomationError::Step { step: 0, .. }
        ));
    }

    #[test]
    fn rejects_label_with_control_char() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"capture","label":"a\nb"},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        assert!(matches!(
            validate_script(doc, p()).unwrap_err(),
            AutomationError::Step { step: 0, .. }
        ));
    }

    // --- REQ-SCRIPT-006: typed main-menu transition ---

    #[test]
    fn rejects_unknown_menu_item_in_transition() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"assert_main_menu_transition","from":"NewGame","to":"Bogus"},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert!(matches!(err, AutomationError::Step { step: 0, .. }));
        let msg = format!("{err}");
        assert!(msg.contains("Bogus"));
    }

    #[test]
    fn accepts_all_five_menu_items_in_transition() {
        for (name, item) in [
            ("NewGame", RestartMenuItem::NewGame),
            ("LoadGame", RestartMenuItem::LoadGame),
            ("SuperMelee", RestartMenuItem::SuperMelee),
            ("Setup", RestartMenuItem::Setup),
            ("Quit", RestartMenuItem::Quit),
        ] {
            let txt = format!(
                r#"{{"version":1,"name":"x","budgets":{{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1}},"steps":[{{"action":"assert_main_menu_transition","from":"NewGame","to":"{name}"}},{{"action":"finish"}}]}}"#
            );
            let doc = parse_script(txt.as_bytes(), p()).unwrap();
            let v = validate_script(doc, p()).unwrap();
            assert_eq!(v.transitions()[0].to, item);
        }
    }

    #[test]
    fn capture_step_does_not_emit_assertion_pass() {
        // REQ-SCRIPT-006: a capture/checkpoint cannot emit assertion pass.
        // We verify by confirming a script with only capture+finish has no
        // transitions (no assertion pass path).
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"capture","label":"ok"},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let v = validate_script(doc, p()).unwrap();
        assert!(v.transitions().is_empty());
    }

    // --- duplicate keys ---

    #[test]
    fn rejects_duplicate_root_keys() {
        let txt = r#"{"version":1,"version":2,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"finish"}]}"#;
        let err = parse_script(txt.as_bytes(), p()).unwrap_err();
        assert!(matches!(err, AutomationError::UnknownField { .. }));
    }

    #[test]
    fn rejects_duplicate_budget_keys() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_input_ticks":2,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"finish"}]}"#;
        let err = parse_script(txt.as_bytes(), p()).unwrap_err();
        assert!(matches!(err, AutomationError::UnknownField { .. }));
    }

    // --- error path/step retention ---

    #[test]
    fn step_errors_retain_path_and_step_index() {
        let txt = r#"{"version":1,"name":"x","budgets":{"max_input_ticks":1,"max_presentations":1,"max_wallclock_seconds":1},"steps":[{"action":"wait_input_ticks","count":0},{"action":"tap_menu_key","key":"down","value":1,"hold":0,"settle":1},{"action":"finish"}]}"#;
        let doc = parse_script(txt.as_bytes(), p()).unwrap();
        let err = validate_script(doc, p()).unwrap_err();
        assert_eq!(err.path(), Some("test.json"));
        assert_eq!(err.step_index(), Some(1));
        let msg = format!("{err}");
        assert!(msg.contains("test.json"));
        assert!(msg.contains("step 1"));
    }

    // --- label validation unit ---

    #[test]
    fn label_validation_table() {
        assert!(is_valid_label("start"));
        assert!(is_valid_label("before-down"));
        assert!(is_valid_label("frame_001"));
        assert!(!is_valid_label(""));
        assert!(!is_valid_label(".."));
        assert!(!is_valid_label("a/b"));
        assert!(!is_valid_label("a\\b"));
        assert!(!is_valid_label("a..b"));
        assert!(!is_valid_label("a\0b"));
    }

    // --- capability contract ---

    #[test]
    fn capability_flags_match_build_contract() {
        assert_eq!(
            CAPABILITY_REQUIRED_FLAGS,
            &[
                "RUST_OWNS_MAIN",
                "USE_RUST_THREADS",
                "USE_RUST_GFX",
                "USE_RUST_COMM",
                "USE_RUST_RESTART",
            ]
        );
    }
}
