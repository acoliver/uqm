# Component 001: Property File Parser (.rmp/.cfg/.key)

Replaces C `PropFile_from_string` in `propfile.c`.

## Pseudocode

```
 1: FUNCTION propfile_parse(data: &str, handler: FnMut(key, value), prefix: Option<&str>)
 2:   LET i = 0
 3:   LET len = data.len()
 4:   WHILE i < len:
 5:     // Skip leading whitespace (including newlines between entries)
 6:     WHILE i < len AND data[i].is_ascii_whitespace():
 7:       i += 1
 8:     IF i >= len: BREAK
 9:
10:     // Check for comment line
11:     IF data[i] == '#':
12:       WHILE i < len AND data[i] != '\n': i += 1
13:       IF i < len: i += 1  // skip newline
14:       CONTINUE
15:
16:     // Extract key: read until '=', '\n', '#', or EOF
17:     LET key_start = i
18:     WHILE i < len AND data[i] != '=' AND data[i] != '\n' AND data[i] != '#':
19:       i += 1
20:
21:     IF i >= len:
22:       log_warning("Bare keyword at EOF")
23:       BREAK
24:
25:     IF data[i] != '=':
26:       log_warning("Key without value")
27:       WHILE i < len AND data[i] != '\n': i += 1
28:       IF i < len: i += 1
29:       CONTINUE
30:
31:     // Trim trailing whitespace from key
32:     LET key_end = i
33:     WHILE key_end > key_start AND data[key_end-1].is_ascii_whitespace():
34:       key_end -= 1
35:     LET key = data[key_start..key_end]
36:
37:     // Skip '='
38:     i += 1
39:
40:     // Skip whitespace after '=' (not past '#' or '\n')
41:     WHILE i < len AND data[i] != '#' AND data[i] != '\n' AND data[i].is_ascii_whitespace():
42:       i += 1
43:     LET value_start = i
44:
45:     // Read value until '#', '\n', or EOF
46:     WHILE i < len AND data[i] != '#' AND data[i] != '\n':
47:       i += 1
48:     LET value_end = i
49:
50:     // Trim trailing whitespace from value
51:     WHILE value_end > value_start AND data[value_end-1].is_ascii_whitespace():
52:       value_end -= 1
53:     LET value = data[value_start..value_end]
54:
55:     // Skip past EOL
56:     WHILE i < len AND data[i] != '\n': i += 1
57:     IF i < len: i += 1
58:
59:     // Apply prefix and invoke handler
60:     IF prefix IS Some(pfx):
61:       LET full_key = format!("{}{}", pfx, key)  // limit to 255 chars
62:       IF full_key.len() > 255: full_key.truncate(255)
63:       handler(full_key, value)
64:     ELSE:
65:       handler(key, value)
66: END FUNCTION
```

## Validation Points
- Line 8: EOF during whitespace skip — clean exit
- Lines 21-23: Bare key at EOF — warning + break
- Lines 25-29: Key without '=' — warning + skip line
- Lines 32-35: Key whitespace trimming (trailing only)
- Lines 50-52: Value whitespace trimming (trailing)
- Line 62: Prefix+key length limit (255 chars, matching C buffer)

## Key Behavioral Differences from Existing Rust Code
- Existing `PropertyFile::from_string` uses `str::lines()` which loses the
  ability to detect bare-key-at-EOF vs key-without-value
- Existing code uppercases keys — new code preserves original case
- Existing code does NOT handle inline `#` comments in values
- Existing code does NOT support prefix mechanism
