//! Randomised comparison of the bank against a plain-vector transcription of
//! the C algorithm in `stringbank.c`. The model stores each chunk's used bytes
//! in a `Vec<u8>` and reports where (chunk, offset) every result lives, so the
//! comparison covers placement, deduplication and refusal, not only contents.

use proptest::collection::vec;
use proptest::prelude::*;

use super::tests::{cstring, locate};
use super::*;

type Place = (usize, usize);

struct Model {
    chunks: Vec<Vec<u8>>,
}

impl Model {
    fn new() -> Self {
        Self {
            chunks: vec![Vec::new()],
        }
    }

    fn add(&mut self, bytes: &[u8]) -> Option<Place> {
        let needed = bytes.len() + 1;
        if needed > STRBANK_CHUNK_SIZE {
            return None;
        }
        let target = match self
            .chunks
            .iter()
            .position(|chunk| needed < STRBANK_CHUNK_SIZE - chunk.len())
        {
            Some(index) => index,
            None => {
                self.chunks.push(Vec::new());
                self.chunks.len() - 1
            }
        };
        let chunk = &mut self.chunks[target];
        let offset = chunk.len();
        chunk.extend_from_slice(bytes);
        chunk.push(0);
        Some((target, offset))
    }

    fn find(&self, bytes: &[u8]) -> Option<Place> {
        self.chunks.iter().enumerate().find_map(|(index, chunk)| {
            let mut offset = 0;
            for entry in chunk.split_inclusive(|byte| *byte == 0) {
                if entry[..entry.len() - 1] == *bytes {
                    return Some((index, offset));
                }
                offset += entry.len();
            }
            None
        })
    }

    fn add_or_find(&mut self, bytes: &[u8]) -> Option<Place> {
        if bytes.len() + 1 > STRBANK_CHUNK_SIZE {
            return None;
        }
        self.find(bytes).or_else(|| self.add(bytes))
    }
}

#[derive(Debug, Clone)]
enum Op {
    Add(Vec<u8>),
    AddOrFind(Vec<u8>),
    Substring {
        text: Vec<u8>,
        n: usize,
        find: bool,
    },
    Split {
        text: Vec<u8>,
        delimiter: u8,
        slots: usize,
    },
}

fn small_text() -> impl Strategy<Value = Vec<u8>> {
    vec(
        prop_oneof![
            Just(b'a'),
            Just(b'b'),
            Just(b'\n'),
            Just(0xC3),
            Just(0xA9),
            Just(0xFF)
        ],
        0..8,
    )
}

/// Lengths around the one-chunk limit so refusal and the strict-fit rule are
/// reached regularly.
fn large_text() -> impl Strategy<Value = Vec<u8>> {
    (
        0usize..STRBANK_CHUNK_SIZE + 20,
        prop_oneof![Just(b'a'), Just(b'b'), Just(b'\n')],
    )
        .prop_map(|(length, byte)| vec![byte; length])
}

fn text() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![8 => small_text(), 2 => large_text()]
}

fn prefix_length() -> impl Strategy<Value = usize> {
    prop_oneof![
        0usize..12,
        990usize..1030,
        any::<u32>().prop_map(|n| n as usize)
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        text().prop_map(Op::Add),
        text().prop_map(Op::AddOrFind),
        (text(), prefix_length(), any::<bool>()).prop_map(|(text, n, find)| Op::Substring {
            text,
            n,
            find
        }),
        (
            text(),
            prop_oneof![Just(b'\n'), Just(b'a'), Just(0xC3), 1u8..=255],
            1usize..8
        )
            .prop_map(|(text, delimiter, slots)| Op::Split {
                text,
                delimiter,
                slots
            }),
    ]
}

fn observed(bank: &StringBank, result: Option<&CStr>) -> Option<(Place, Vec<u8>)> {
    result.map(|stored| (locate(bank, stored), stored.to_bytes().to_vec()))
}

fn expected(place: Option<Place>, bytes: &[u8]) -> Option<(Place, Vec<u8>)> {
    place.map(|place| (place, bytes.to_vec()))
}

fn check_op(bank: &StringBank, model: &mut Model, op: &Op) -> Result<(), TestCaseError> {
    match op {
        Op::Add(bytes) => {
            let got = observed(bank, bank.add_string(&cstring(bytes)));
            let place = model.add(bytes);
            prop_assert_eq!(got, expected(place, bytes));
        }
        Op::AddOrFind(bytes) => {
            let got = observed(bank, bank.add_or_find_string(&cstring(bytes)));
            let place = model.add_or_find(bytes);
            prop_assert_eq!(got, expected(place, bytes));
        }
        Op::Substring { text, n, find } => {
            let head = &text[..(*n).min(text.len())];
            let source = cstring(text);
            let (got, place) = if *find {
                (
                    bank.add_or_find_substring(&source, *n),
                    model.add_or_find(head),
                )
            } else {
                (bank.add_substring(&source, *n), model.add(head))
            };
            prop_assert_eq!(observed(bank, got), expected(place, head));
        }
        Op::Split {
            text,
            delimiter,
            slots,
        } => {
            let source = cstring(text);
            let got: Vec<_> = bank
                .split_string(
                    &source,
                    NonZeroU8::new(*delimiter).unwrap(),
                    NonZeroUsize::new(*slots).unwrap(),
                )
                .map(|piece| observed(bank, piece))
                .collect();
            let want: Vec<_> = text
                .splitn(*slots, |byte| byte == delimiter)
                .map(|piece| {
                    let place = model.add_or_find(piece);
                    expected(place, piece)
                })
                .collect();
            prop_assert_eq!(got, want);
        }
    }
    Ok(())
}

proptest! {
    #[test]
    fn bank_matches_the_c_algorithm_and_keeps_every_pointer_stable(
        ops in vec(op(), 1..40)
    ) {
        let bank = StringBank::create();
        let mut model = Model::new();
        for op in &ops {
            check_op(&bank, &mut model, op)?;
            // The bank's chunks hold exactly what the model's chunks hold, so
            // every earlier string is still where it was and unchanged.
            let held: Vec<Vec<u8>> = bank.chunks().map(|chunk| chunk.used().to_vec()).collect();
            prop_assert_eq!(held, model.chunks.clone());
        }
    }

    #[test]
    fn split_matches_splitn_for_any_bytes(
        text in vec(any::<u8>().prop_filter("no NUL", |byte| *byte != 0), 0..200),
        delimiter in 1u8..=255,
        slots in 1usize..10,
    ) {
        let bank = StringBank::create();
        let source = cstring(&text);
        let got: Vec<Vec<u8>> = bank
            .split_string(&source, NonZeroU8::new(delimiter).unwrap(), NonZeroUsize::new(slots).unwrap())
            .map(|piece| piece.unwrap().to_bytes().to_vec())
            .collect();
        let want: Vec<Vec<u8>> = text
            .splitn(slots, |byte| *byte == delimiter)
            .map(<[u8]>::to_vec)
            .collect();
        prop_assert_eq!(got, want);
    }

    #[test]
    fn add_or_find_is_idempotent(texts in vec(small_text(), 1..30)) {
        let bank = StringBank::create();
        for text in &texts {
            let first = bank.add_or_find_string(&cstring(text)).unwrap().as_ptr();
            let second = bank.add_or_find_string(&cstring(text)).unwrap().as_ptr();
            prop_assert_eq!(first, second);
        }
    }
}
