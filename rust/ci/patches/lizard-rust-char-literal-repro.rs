fn character_literals() -> (char, [char; 5]) {
    ('Z', ['0', '中', '\n', '\'', '\\'])
}

fn lifetimes<'a, 'r#type>(ordinary: &'a str, raw: &'r#type str) -> (&'a str, &'r#type str) {
    'label: loop {
        break 'label (ordinary, raw);
    }
}

fn following_function(value: i32) -> i32 {
    value + 1
}
