// Property File Parser
// Parses simple key=value property files
//
// @plan PLAN-20260224-RES-SWAP.P03
// @requirement REQ-RES-018, REQ-RES-R007, REQ-RES-006-012

/// Parse a property file string, invoking `handler` for each key-value pair.
///
/// This matches the C `PropFile_from_string` behavior: preserves key case, handles inline `#`
/// comments, and supports an optional key prefix.
///
/// # Arguments
/// * `data` - The property file content to parse
/// * `handler` - Callback invoked with `(key, value)` for each entry
/// * `prefix` - Optional prefix to prepend to all keys (total key length capped at 255)
///
/// @plan PLAN-20260224-RES-SWAP.P05
/// @requirement REQ-RES-018, REQ-RES-R007, REQ-RES-006-012
pub fn parse_propfile(data: &str, handler: &mut dyn FnMut(&str, &str), prefix: Option<&str>) {
    const NL: u8 = b'\x0A';
    const HASH: u8 = b'#';
    const EQ: u8 = b'=';

    let bytes = data.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        if bytes[i] == HASH {
            while i < len && bytes[i] != NL {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        let key_start = i;
        while i < len && bytes[i] != EQ && bytes[i] != NL && bytes[i] != HASH {
            i += 1;
        }

        if i >= len {
            log::warn!("Bare keyword at EOF");
            break;
        }

        if bytes[i] != EQ {
            log::warn!("Key without value");
            while i < len && bytes[i] != NL {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        let mut key_end = i;
        while key_end > key_start && bytes[key_end - 1].is_ascii_whitespace() {
            key_end -= 1;
        }
        let key = &data[key_start..key_end];

        i += 1;

        while i < len && bytes[i] != HASH && bytes[i] != NL && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let value_start = i;

        while i < len && bytes[i] != HASH && bytes[i] != NL {
            i += 1;
        }
        let mut value_end = i;

        while value_end > value_start && bytes[value_end - 1].is_ascii_whitespace() {
            value_end -= 1;
        }
        let value = &data[value_start..value_end];

        while i < len && bytes[i] != NL {
            i += 1;
        }
        if i < len {
            i += 1;
        }

        if let Some(pfx) = prefix {
            let mut full_key = format!("{}{}", pfx, key);
            if full_key.len() > 255 {
                full_key.truncate(255);
            }
            handler(&full_key, value);
        } else {
            handler(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // @plan PLAN-20260224-RES-SWAP.P04
    // @requirement REQ-RES-006-012, REQ-RES-018, REQ-RES-R007

    #[test]
    fn test_propfile_basic_keyvalue() {
        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            "key = GFXRES:path/to/file",
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            None,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "key");
        assert_eq!(calls[0].1, "GFXRES:path/to/file");
    }

    #[test]
    fn test_propfile_preserves_key_case() {
        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            "comm.Arilou.Graphics = GFXRES:path",
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            None,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "comm.Arilou.Graphics");
    }

    #[test]
    fn test_propfile_comment_line() {
        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            "# this is a comment
key = value",
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            None,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "key");
        assert_eq!(calls[0].1, "value");
    }

    #[test]
    fn test_propfile_inline_comment() {
        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            "key = value # inline comment",
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            None,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "key");
        assert_eq!(calls[0].1, "value");
    }

    #[test]
    fn test_propfile_whitespace_trimming() {
        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            "  key  =  value  ",
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            None,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "key");
        assert_eq!(calls[0].1, "value");
    }

    #[test]
    fn test_propfile_blank_lines() {
        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            "


key = value

",
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            None,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "key");
        assert_eq!(calls[0].1, "value");
    }

    #[test]
    fn test_propfile_key_without_value() {
        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            "barekey
key = value",
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            None,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "key");
        assert_eq!(calls[0].1, "value");
    }

    #[test]
    fn test_propfile_bare_key_at_eof() {
        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            "barekey",
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            None,
        );
        assert_eq!(calls.len(), 0);
    }

    #[test]
    fn test_propfile_prefix_prepended() {
        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            "sfxvol = INT32:20",
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            Some("config."),
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "config.sfxvol");
        assert_eq!(calls[0].1, "INT32:20");
    }

    #[test]
    fn test_propfile_null_prefix() {
        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            "key = value",
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            None,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "key");
        assert_eq!(calls[0].1, "value");
    }

    #[test]
    fn test_propfile_prefix_length_limit() {
        let long_prefix = "a".repeat(250);
        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            "longkey123 = value",
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            Some(&long_prefix),
        );
        assert_eq!(calls.len(), 1);
        // 250 prefix + 10 key = 260, should be truncated to 255
        assert!(calls[0].0.len() <= 255);
        assert!(calls[0].0.starts_with(&long_prefix[..245]));
    }

    #[test]
    fn test_propfile_multiple_entries() {
        let input = "\
comm.Arilou.Graphics = GFXRES:base/comm/arilou/arilou.ani
comm.Arilou.Speech = SNDRES:base/comm/arilou/arilou.snd
comm.Arilou.Conversation = STRTAB:base/comm/arilou/arilou.txt
config.sfxvol = INT32:20
config.smooth = BOOLEAN:true";

        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            input,
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            None,
        );

        assert_eq!(calls.len(), 5);
        assert_eq!(calls[0].0, "comm.Arilou.Graphics");
        assert_eq!(calls[0].1, "GFXRES:base/comm/arilou/arilou.ani");
        assert_eq!(calls[1].0, "comm.Arilou.Speech");
        assert_eq!(calls[1].1, "SNDRES:base/comm/arilou/arilou.snd");
        assert_eq!(calls[2].0, "comm.Arilou.Conversation");
        assert_eq!(calls[2].1, "STRTAB:base/comm/arilou/arilou.txt");
        assert_eq!(calls[3].0, "config.sfxvol");
        assert_eq!(calls[3].1, "INT32:20");
        assert_eq!(calls[4].0, "config.smooth");
        assert_eq!(calls[4].1, "BOOLEAN:true");
    }

    #[test]
    fn test_propfile_real_rmp_content() {
        let rmp_content = "\
# UQM Resource Map
comm.Arilou.Graphics = GFXRES:base/comm/arilou/arilou.ani
comm.Arilou.Speech = SNDRES:base/comm/arilou/arilou.snd
comm.Arilou.Conversation = STRTAB:base/comm/arilou/arilou.txt
comm.Chmmr.Graphics = GFXRES:base/comm/chmmr/chmmr.ani
comm.Chmmr.Speech = SNDRES:base/comm/chmmr/chmmr.snd
comm.Chmmr.Conversation = STRTAB:base/comm/chmmr/chmmr.txt
config.textSpeed = INT32:40
config.3doMusic = BOOLEAN:true
config.resolutionFactor = INT32:0";

        let mut calls: Vec<(String, String)> = Vec::new();
        parse_propfile(
            rmp_content,
            &mut |k, v| {
                calls.push((k.to_string(), v.to_string()));
            },
            None,
        );

        assert_eq!(calls.len(), 9);
        // Verify first entry
        assert_eq!(calls[0].0, "comm.Arilou.Graphics");
        assert_eq!(calls[0].1, "GFXRES:base/comm/arilou/arilou.ani");
        // Verify a config entry (index 6: textSpeed is the 7th entry after 6 comm entries)
        assert_eq!(calls[6].0, "config.textSpeed");
        assert_eq!(calls[6].1, "INT32:40");
        // Verify last entry
        assert_eq!(calls[8].0, "config.resolutionFactor");
        assert_eq!(calls[8].1, "INT32:0");
    }
}
