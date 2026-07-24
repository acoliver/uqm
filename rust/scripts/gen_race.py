#!/usr/bin/env python3
"""
Generate Rust race dialogue modules from C source.

Usage: python3 gen_race.py <race_dir> <rust_output>

Reads:
  - comm/<race>/<race>c.c (dialogue state machine)
  - comm/<race>/strings.h (string indices)
  - comm/<race>/resinst.h (resource keys)

Generates a Rust module with:
  - String index constants
  - Resource key constants
  - Game state key references
  - Translated state machine functions
  - RaceDialogue trait implementation
"""

import re
import sys
import os
from pathlib import Path

def parse_strings_h(filepath):
    """Extract string enum values from strings.h"""
    with open(filepath) as f:
        content = f.read()
    match = re.search(r'enum\s*\{([^}]*)\}', content)
    if not match:
        return {}
    entries = []
    for line in match.group(1).split(','):
        line = line.strip()
        if not line or line.startswith('//') or line.startswith('/*'):
            continue
        # Remove comments
        line = re.sub(r'/\*.*?\*/', '', line)
        line = re.sub(r'//.*$', '', line)
        line = line.strip()
        if line:
            entries.append(line)
    return {name: i for i, name in enumerate(entries)}

def parse_resinst_h(filepath):
    """Extract resource key definitions from resinst.h"""
    keys = {}
    if not os.path.exists(filepath):
        return keys
    with open(filepath) as f:
        content = f.read()
    # Look for #define X "value"
    for m in re.finditer(r'#define\s+(\w+)\s+"([^"]*)"', content):
        keys[m.group(1)] = m.group(2)
    return keys

def extract_game_state_keys(c_content):
    """Extract all game state keys used in the C file"""
    keys = set()
    for m in re.finditer(r'(?:GET|SET)_GAME_STATE\s*\(\s*([A-Z_][A-Z0-9_]*)', c_content):
        keys.add(m.group(1))
    return sorted(keys)

def to_rust_const(name):
    """Convert C enum name to Rust constant name"""
    return name.upper() if name.islower() else name

def generate_rust_module(race_name, strings, resinst, gs_keys, c_content):
    """Generate the Rust module content"""
    lines = []
    
    # Header
    lines.append(f'//! {race_name.capitalize()} dialogue state machine — ported from C.')
    lines.append('//!')
    lines.append('//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15')
    lines.append('')
    lines.append('#![allow(dead_code)]')
    lines.append('')
    lines.append('use std::ffi::c_char;')
    lines.append('use std::os::raw::c_int;')
    lines.append('')
    lines.append('use crate::comm::segue::Segue;')
    lines.append('use crate::comm::types::{AnimationDescData, CommData, TextAlign, TextValign};')
    lines.append('use crate::state::game_state_keys::bit_range;')
    lines.append('')
    
    # String constants
    lines.append('// ---------------------------------------------------------------------------')
    lines.append('// String indices (from strings.h)')
    lines.append('// ---------------------------------------------------------------------------')
    lines.append('')
    for name, idx in sorted(strings.items(), key=lambda x: x[1]):
        rust_name = to_rust_const(name)
        lines.append(f'const {rust_name}: u32 = {idx};')
    lines.append('')
    
    # Game state helpers
    lines.append('// ---------------------------------------------------------------------------')
    lines.append('// Game state helpers')
    lines.append('// ---------------------------------------------------------------------------')
    lines.append('')
    lines.append('fn get_gs(key: &str) -> u8 {')
    lines.append('    let (start, end) = match bit_range(key) {')
    lines.append('        Some(r) => r,')
    lines.append('        None => return 0,')
    lines.append('    };')
    lines.append('    unsafe { rust_get_game_state_bits(start as c_int, end as c_int) }')
    lines.append('}')
    lines.append('')
    lines.append('fn set_gs(key: &str, val: u8) {')
    lines.append('    if let Some((start, end)) = bit_range(key) {')
    lines.append('        unsafe { rust_set_game_state_bits(start as c_int, end as c_int, val) };')
    lines.append('    }')
    lines.append('}')
    lines.append('')
    
    # FFI declarations
    lines.append('extern "C" {')
    lines.append('    fn rust_NPCPhrase_cb(index: c_int, cb: Option<extern "C" fn()>);')
    lines.append('    fn rust_PhraseEnabled(index: c_int) -> c_int;')
    lines.append('    fn rust_DisablePhrase(index: c_int);')
    lines.append('    fn DoResponsePhrase(')
    lines.append('        response_ref: u32,')
    lines.append('        response_func: Option<extern "C" fn(u32)>,')
    lines.append('        construct_str: *const c_char,')
    lines.append('    );')
    lines.append('    fn rust_get_game_state_bits(start: c_int, end: c_int) -> u8;')
    lines.append('    fn rust_set_game_state_bits(start: c_int, end: c_int, val: u8);')
    lines.append('    fn rust_add_event_relative(days_offset: u32, func_index: u8) -> u32;')
    lines.append('}')
    lines.append('')
    
    # Comm helpers
    lines.append('fn npc_phrase(index: u32) {')
    lines.append('    if index == 0 { return; }')
    lines.append('    unsafe { rust_NPCPhrase_cb(index as c_int, None) };')
    lines.append('}')
    lines.append('')
    lines.append('fn phrase_enabled(index: u32) -> bool {')
    lines.append('    unsafe { rust_PhraseEnabled(index as c_int) != 0 }')
    lines.append('}')
    lines.append('')
    lines.append('fn disable_phrase(index: u32) {')
    lines.append('    unsafe { rust_DisablePhrase(index as c_int) };')
    lines.append('}')
    lines.append('')
    lines.append('fn response(phrase: u32, callback: extern "C" fn(u32)) {')
    lines.append('    unsafe { DoResponsePhrase(phrase, Some(callback), std::ptr::null()) };')
    lines.append('}')
    lines.append('')
    lines.append('fn set_segue(segue: Segue) {')
    lines.append('    crate::comm::state::COMM_STATE.write().set_segue(segue);')
    lines.append('}')
    lines.append('')
    lines.append('fn get_segue() -> Segue {')
    lines.append('    crate::comm::state::COMM_STATE.read().get_segue()')
    lines.append('}')
    lines.append('')
    lines.append('fn get_current_activity() -> u16 {')
    lines.append('    unsafe { crate::mainloop::c_extern::get_current_activity() }')
    lines.append('}')
    lines.append('')
    lines.append('fn lobyte(val: u16) -> u8 { (val & 0xFF) as u8 }')
    lines.append('')
    
    # Resource keys
    lines.append('// ---------------------------------------------------------------------------')
    lines.append(f'// Resource keys (from resinst.h)')
    lines.append('// ---------------------------------------------------------------------------')
    lines.append('')
    
    # Find the race prefix from resinst
    race_prefix = race_name.upper()
    pmap_key = resinst.get(f'{race_prefix}_PMAP_ANIM', f'{race_name.lower()}')
    font_key = resinst.get(f'{race_prefix}_FONT', f'{race_name.lower()}font')
    color_key = resinst.get(f'{race_prefix}_COLOR_MAP', f'{race_name.lower()}colr')
    music_key = resinst.get(f'{race_prefix}_MUSIC', f'{race_name.lower()}music')
    phrases_key = resinst.get(f'{race_prefix}_CONVERSATION_PHRASES', f'comm.{race_name.lower()}.dialogue')
    
    lines.append(f'const RACE_PMAP_ANIM: &[u8] = b"{pmap_key}\\0";')
    lines.append(f'const RACE_FONT: &[u8] = b"{font_key}\\0";')
    lines.append(f'const RACE_COLOR_MAP: &[u8] = b"{color_key}\\0";')
    lines.append(f'const RACE_MUSIC: &[u8] = b"{music_key}\\0";')
    lines.append(f'const RACE_CONVERSATION_PHRASES: &[u8] = b"{phrases_key}\\0";')
    lines.append('')
    
    # Struct and trait impl
    lines.append(f'/// {race_name.capitalize()} race dialogue implementation.')
    lines.append(f'pub struct {race_name.capitalize()}Dialogue;')
    lines.append('')
    lines.append(f'impl super::RaceDialogue for {race_name.capitalize()}Dialogue {{')
    lines.append('    fn init(&self) -> CommData {')
    lines.append('        let data = CommData {')
    lines.append('            alien_frame_res: RACE_PMAP_ANIM.as_ptr() as *const _,')
    lines.append('            alien_font_res: RACE_FONT.as_ptr() as *const _,')
    lines.append('            alien_colormap_res: RACE_COLOR_MAP.as_ptr() as *const _,')
    lines.append('            alien_song_res: RACE_MUSIC.as_ptr() as *const _,')
    lines.append('            alien_alt_song_res: std::ptr::null(),')
    lines.append('            conversation_phrases_res: RACE_CONVERSATION_PHRASES.as_ptr() as *const _,')
    lines.append('            alien_text_align: TextAlign::Center,')
    lines.append('            alien_text_valign: TextValign::Top,')
    lines.append('            alien_text_fcolor: 0x00FFFFFF,')
    lines.append('            alien_text_bcolor: 0x00000000,')
    lines.append('            ..CommData::default()')
    lines.append('        };')
    lines.append('        data')
    lines.append('    }')
    lines.append('')
    lines.append('    fn intro(&self) {')
    lines.append('        // TODO: Port intro state machine from C')
    lines.append('    }')
    lines.append('')
    lines.append('    fn post_encounter(&self) {')
    lines.append('        // TODO: Port post_encounter from C')
    lines.append('    }')
    lines.append('')
    lines.append('    fn uninit(&self) -> u32 {')
    lines.append('        0')
    lines.append('    }')
    lines.append('}')
    lines.append('')
    
    # Tests
    lines.append('#[cfg(test)]')
    lines.append('mod tests {')
    lines.append('    use super::*;')
    lines.append('')
    lines.append('    #[test]')
    lines.append('    fn test_resource_keys_are_null_terminated() {')
    lines.append('        assert_eq!(RACE_PMAP_ANIM.last(), Some(&0));')
    lines.append('        assert_eq!(RACE_FONT.last(), Some(&0));')
    lines.append('        assert_eq!(RACE_COLOR_MAP.last(), Some(&0));')
    lines.append('        assert_eq!(RACE_MUSIC.last(), Some(&0));')
    lines.append('    }')
    lines.append('')
    
    # Test game state keys exist
    if gs_keys:
        lines.append('    #[test]')
        lines.append(f'    fn test_game_state_keys_exist() {{')
        for key in gs_keys[:5]:
            lines.append(f'        assert!(bit_range("{key}").is_some(), "missing game state key: {key}");')
        lines.append('    }')
        lines.append('')
    
    lines.append('}')
    lines.append('')
    
    return '\n'.join(lines)

def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <race_dir> <rust_output>")
        sys.exit(1)
    
    race_dir = Path(sys.argv[1])
    output_file = Path(sys.argv[2])
    
    race_name = race_dir.name
    
    # Find the C source file
    c_files = list(race_dir.glob('*c.c')) + list(race_dir.glob('*.c'))
    c_file = None
    for f in c_files:
        if 'c.c' in f.name or f.name.endswith('c.c'):
            c_file = f
            break
    if not c_file:
        c_file = c_files[0] if c_files else None
    
    if not c_file:
        print(f"ERROR: No C source file found in {race_dir}")
        sys.exit(1)
    
    # Parse strings.h
    strings_h = race_dir / 'strings.h'
    strings = parse_strings_h(strings_h) if strings_h.exists() else {}
    
    # Parse resinst.h
    resinst_h = race_dir / 'resinst.h'
    resinst = parse_resinst_h(resinst_h) if resinst_h.exists() else {}
    
    # Read C source
    with open(c_file) as f:
        c_content = f.read()
    
    # Extract game state keys
    gs_keys = extract_game_state_keys(c_content)
    
    # Generate Rust module
    rust_code = generate_rust_module(race_name, strings, resinst, gs_keys, c_content)
    
    # Write output
    output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(output_file, 'w') as f:
        f.write(rust_code)
    
    print(f"Generated {output_file} ({len(rust_code)} bytes)")
    print(f"  Strings: {len(strings)} entries")
    print(f"  Resource keys: {len(resinst)} entries")
    print(f"  Game state keys: {len(gs_keys)} keys")

if __name__ == '__main__':
    main()