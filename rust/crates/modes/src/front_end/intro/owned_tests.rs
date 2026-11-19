use super::{args::Arguments, parse_rgb, text};

#[test]
fn decimal_arguments_preserve_scan_prefixes_and_stop_on_invalid_input() {
    let mut args = Arguments::new(b" \t-12 +34 56tail 78");
    assert_eq!(args.integer(), Some(-12));
    assert_eq!(args.integer(), Some(34));
    assert_eq!(args.integer(), Some(56));
    assert_eq!(args.integer(), None);
    assert_eq!(args.remaining(), b"tail 78");
}

#[test]
fn bounded_words_leave_the_unconsumed_bytes_for_the_next_scan() {
    let mut args = Arguments::new(b"  abcdef\tsecond\nthird");
    assert_eq!(args.word(3), Some(b"abc".as_slice()));
    assert_eq!(args.word(15), Some(b"def".as_slice()));
    assert_eq!(args.word(15), Some(b"second".as_slice()));
    assert_eq!(args.word(15), Some(b"third".as_slice()));
    assert_eq!(args.word(15), None);
}

#[test]
fn script_colors_keep_hex_prefix_sign_and_zero_alpha() {
    for (bytes, expected) in [
        (b" 0x1234ab trailing".as_slice(), (0x12, 0x34, 0xab, 0)),
        (b"ff00ff!".as_slice(), (255, 0, 255, 0)),
        (b"-1".as_slice(), (255, 255, 255, 0)),
        (b"0X0010ef".as_slice(), (0, 16, 239, 0)),
    ] {
        let color = parse_rgb(bytes).unwrap();
        assert_eq!((color.r, color.g, color.b, color.a), expected);
    }
    assert!(parse_rgb(b"invalid").is_none());
    assert!(parse_rgb(b"").is_none());
}

#[test]
fn owned_text_preserves_empty_rows_and_bounds_without_borrowing_the_buffer() {
    let mut source = b"first\n\nlast\n".to_vec();
    let lines = text::owned_lines(&source, 15);
    source.fill(b'x');
    assert_eq!(
        lines.iter().map(|line| line.to_bytes()).collect::<Vec<_>>(),
        vec![b"first".as_slice(), b"", b"last"]
    );
    let lines = text::owned_lines(b"a\nb\nc", 2);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1].to_bytes(), b"b");
    assert!(text::owned_lines(b"", 15).is_empty());
    assert!(text::owned_lines(b"a", 0).is_empty());
}
