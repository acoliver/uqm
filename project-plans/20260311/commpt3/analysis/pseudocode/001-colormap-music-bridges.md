# Pseudocode 001: Colormap and Music Bridge Fixes

Plan ID: `PLAN-20260325-COMMPT3`
Requirements: REQ-CM-001, REQ-CM-002, REQ-CM-003, REQ-MU-001, REQ-MU-002, REQ-MU-003
Implementation Phase: P03

## C Bridge: c_SetColorMapFromCommData (rust_comm.c)

```
01: FUNCTION c_SetColorMapFromCommData()
02:   colormap = CommData.AlienColorMap
03:   IF colormap == 0 THEN
04:     RETURN                     // no colormap loaded, graceful no-op
05:   END IF
06:   address = GetColorMapAddress(colormap)
07:   SetColorMap(address)
08: END FUNCTION
```

## C Bridge: c_PlayAlienMusic (rust_comm.c)

```
09: FUNCTION c_PlayAlienMusic()
10:   song = CommData.AlienSong
11:   IF song == 0 THEN
12:     RETURN                     // no music loaded, graceful no-op
13:   END IF
14:   PlayMusic(song, TRUE, 1)     // looping=TRUE, priority=1
15: END FUNCTION
```

## Rust: set_colormap fix (talk_segue.rs, lines 997-1009)

```
16: FUNCTION set_colormap(state: &mut CommState)
17:   #[cfg(not(test))]
18:     CALL c_SetColorMapFromCommData()   // no arguments needed
19:   #[cfg(test)]
20:     NO-OP (suppress unused state)
21: END FUNCTION
```

## Rust: play_alien_music fix (talk_segue.rs, lines 939-951)

```
22: FUNCTION play_alien_music(state: &mut CommState)
23:   #[cfg(not(test))]
24:     CALL c_PlayAlienMusic()            // no arguments needed
25:   #[cfg(test)]
26:     NO-OP (suppress unused state)
27: END FUNCTION
```

## Rust: extern block changes (talk_segue.rs, c_bridge module)

```
28: REMOVE declaration: c_SetColorMap(colormap: *mut c_void)    // line 66
29: REMOVE declaration: c_PlayMusic(song: *mut c_void, ...)     // line 60
30: ADD declaration: c_SetColorMapFromCommData()                 // void, no args
31: ADD declaration: c_PlayAlienMusic()                          // void, no args
```

## Requirement-to-Line Contracts

| Requirement | Pseudocode Lines | Contract |
|---|---|---|
| REQ-CM-001 | 01-08, 16-21 | `c_SetColorMapFromCommData()` called (not null pointer) |
| REQ-CM-002 | 03-04 | No-op when `AlienColorMap == 0` |
| REQ-CM-003 | 02 | Reads current `CommData.AlienColorMap` each call (no cache) |
| REQ-MU-001 | 09-15, 22-27 | `c_PlayAlienMusic()` called (not null pointer) |
| REQ-MU-002 | 11-12 | No-op when `AlienSong == 0` |
| REQ-MU-003 | 14, 22-27 | Music started via `PlayMusic` before first `AlienTalkSegue` frame |
| REQ-SM-001 | 28-31 | "for now" comment removed, old declarations removed |

## Validation Points

- Line 03/11: Null-handle guard prevents crash on missing resource
- Line 06: `GetColorMapAddress` extracts the address from the opaque handle
  (may be a macro — the C bridge handles this, Rust never sees the handle)
- Line 14: `TRUE` = 1 (C BOOLEAN), matches `comm.c` HailAlien behavior
- Lines 28-29: Verify no other callers of `c_SetColorMap` or `c_PlayMusic`
  exist in talk_segue.rs before removing

## Error Handling

- If `CommData.AlienColorMap` is 0, encounter runs without colormap (alien
  portrait shows with default/identity colormap)
- If `CommData.AlienSong` is 0, encounter runs silently (no background music)
- Both conditions are recoverable — encounter continues normally

## Side Effects

- `SetColorMap` modifies the global colormap state for the current context
- `PlayMusic` starts the music subsystem playing the alien song on loop
- Both match the C HailAlien behavior exactly
