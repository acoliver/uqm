# Phase 07: Color Parser Fix — TDD

## Phase ID
`PLAN-20260224-RES-SWAP.P07`

## Prerequisites
- Required: Phase 06a (Color Stub Verification) completed

## Requirements Implemented (Expanded)

### REQ-RES-066-074: Color Parsing and Serialization — Tests

## Implementation Tasks

### Tests to write

#### Color parsing tests
```
test_parse_rgb_decimal
  Input: "rgb(255, 128, 64)"
  Expected: (255, 128, 64, 255)

test_parse_rgb_hex
  Input: "rgb(0x1a, 0x00, 0x1a)"
  Expected: (0x1a, 0x00, 0x1a, 0xff)

test_parse_rgb_hex_uppercase
  Input: "rgb(0xFF, 0x80, 0x40)"
  Expected: (255, 128, 64, 255)

test_parse_rgb_octal
  Input: "rgb(0377, 0200, 0100)"
  Expected: (255, 128, 64, 255)

test_parse_rgba_decimal
  Input: "rgba(255, 0, 0, 128)"
  Expected: (255, 0, 0, 128)

test_parse_rgba_hex
  Input: "rgba(0xff, 0x00, 0x00, 0x80)"
  Expected: (255, 0, 0, 128)

test_parse_rgb15_basic
  Input: "rgb15(31, 0, 0)"
  Expected: CC5TO8(31)=255, CC5TO8(0)=0, CC5TO8(0)=0, alpha=255
  Note: CC5TO8(x) = (x << 3) | (x >> 2)

test_parse_rgb15_mid
  Input: "rgb15(16, 16, 16)"
  Expected: CC5TO8(16)=132, CC5TO8(16)=132, CC5TO8(16)=132, alpha=255

test_parse_rgb15_zero
  Input: "rgb15(0, 0, 0)"
  Expected: (0, 0, 0, 255)

test_parse_rgb_with_whitespace
  Input: "rgb( 255 , 128 , 64 )"
  Expected: (255, 128, 64, 255)

test_parse_rgb_clamp_negative
  Input: "rgb(-1, 0, 0)"
  Expected: (0, 0, 0, 255) with warning

test_parse_rgb_clamp_overflow
  Input: "rgb(256, 0, 0)"
  Expected: (255, 0, 0, 255) with warning

test_parse_rgb15_clamp_overflow
  Input: "rgb15(32, 0, 0)"
  Expected: CC5TO8(31), 0, 0, 255 with warning

test_parse_invalid_format
  Input: "#FF0000"
  Expected: Error, (0, 0, 0, 0)

test_parse_empty_string
  Input: ""
  Expected: Error, (0, 0, 0, 0)

test_parse_garbage
  Input: "not a color"
  Expected: Error, (0, 0, 0, 0)
```

#### Color serialization tests
```
test_serialize_opaque_color
  Input: (0x1a, 0x00, 0x1a, 0xff)
  Expected: "rgb(0x1a, 0x00, 0x1a)"

test_serialize_transparent_color
  Input: (0x1a, 0x00, 0x1a, 0x80)
  Expected: "rgba(0x1a, 0x00, 0x1a, 0x80)"

test_serialize_fully_transparent
  Input: (255, 0, 0, 0)
  Expected: "rgba(0xff, 0x00, 0x00, 0x00)"

test_serialize_black_opaque
  Input: (0, 0, 0, 255)
  Expected: "rgb(0x00, 0x00, 0x00)"

test_serialize_roundtrip
  Parse "rgb(0x1a, 0x00, 0x1a)" → serialize → "rgb(0x1a, 0x00, 0x1a)"
  Parse "rgba(0xff, 0x00, 0x00, 0x80)" → serialize → "rgba(0xff, 0x00, 0x00, 0x80)"
```

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features 2>&1 | grep -E "test.*FAILED|test result"
```

## Structural Verification Checklist
- [ ] All listed tests exist
- [ ] Tests compile
- [ ] Plan markers present

## Semantic Verification Checklist
- [ ] Tests fail with stub implementation (RED)
- [ ] Tests cover all three formats (rgb, rgba, rgb15)
- [ ] Tests cover hex, decimal, octal input
- [ ] Tests cover clamping for both 8-bit and 5-bit
- [ ] Tests cover serialization format exactness
- [ ] Tests cover roundtrip (parse → serialize → same string)

## Success Criteria
- [ ] All tests compile and FAIL (RED)

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P07.md`
