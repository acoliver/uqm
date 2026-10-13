//! `fixture-spec.md` section C: the array copies against a `Vec` model, and
//! the size arithmetic against 128-bit arithmetic.

use std::ffi::c_int;

use proptest::prelude::*;

use super::arrays::{plan_exclude, plan_insert, ArrayError};
use super::c_abi::{exclude_array, insert_array};
use super::c_abi_tests::{take, Padded};
use crate::io::paths::lifetime_probe::Probe;

#[derive(Debug, Clone)]
struct Case {
    stride: usize,
    data: Vec<u8>,
    element: Vec<u8>,
    pos: usize,
}

impl Case {
    fn old_len(&self) -> usize {
        self.data.len() / self.stride
    }
}

fn cases() -> impl Strategy<Value = Case> {
    (1usize..=9, 0usize..=24).prop_flat_map(|(stride, old)| {
        (
            proptest::collection::vec(any::<u8>(), old * stride),
            proptest::collection::vec(any::<u8>(), stride),
            0..=old,
        )
            .prop_map(move |(data, element, pos)| Case {
                stride,
                data,
                element,
                pos,
            })
    })
}

fn insert(case: &Case, data: &[u8]) -> Vec<u8> {
    let probe = Probe::start();
    let source = Padded::new(data);
    let block = unsafe {
        insert_array(
            source.ptr(),
            data.len() / case.stride,
            case.pos as c_int,
            case.element.as_ptr().cast(),
            case.stride,
        )
    };
    let got = take(&probe, block, data.len() + case.stride);
    source.assert_canaries_intact();
    probe.assert_balanced();
    got
}

fn exclude(stride: usize, data: &[u8], start: usize, count: usize) -> Vec<u8> {
    let probe = Probe::start();
    let source = Padded::new(data);
    let block = unsafe {
        exclude_array(
            source.ptr(),
            data.len() / stride,
            start as c_int,
            count as c_int,
            stride,
        )
    };
    let got = take(&probe, block, data.len() - count * stride);
    source.assert_canaries_intact();
    probe.assert_balanced();
    got
}

proptest! {
    #[test]
    fn insert_matches_a_splice_model(case in cases()) {
        let at = case.pos * case.stride;
        let model = [&case.data[..at], &case.element, &case.data[at..]].concat();
        prop_assert_eq!(insert(&case, &case.data), model);
    }

    #[test]
    fn exclude_matches_a_drain_model(case in cases(), count in 0usize..=24) {
        let old = case.old_len();
        let start = case.pos;
        let count = count.min(old - start);
        let mut model = case.data.clone();
        model.drain(start * case.stride..(start + count) * case.stride);
        prop_assert_eq!(exclude(case.stride, &case.data, start, count), model);
    }

    #[test]
    fn excluding_what_was_inserted_round_trips(case in cases()) {
        let inserted = insert(&case, &case.data);
        prop_assert_eq!(exclude(case.stride, &inserted, case.pos, 1), case.data);
    }

    #[test]
    fn inserting_what_was_excluded_round_trips(case in cases()) {
        prop_assume!(case.old_len() > 0);
        let start = case.pos.min(case.old_len() - 1);
        let removed = case.data[start * case.stride..(start + 1) * case.stride].to_vec();
        let shorter = exclude(case.stride, &case.data, start, 1);
        let again = Case { element: removed, pos: start, ..case.clone() };
        prop_assert_eq!(insert(&again, &shorter), case.data);
    }

    #[test]
    fn insert_sizes_overflow_exactly_when_128_bit_arithmetic_does(
        old in prop_oneof![any::<usize>(), (0usize..64), Just(usize::MAX), Just(usize::MAX / 2)],
        stride in prop_oneof![any::<usize>(), (0usize..64), Just(usize::MAX)],
    ) {
        let wide = (old as u128 + 1) * stride as u128;
        match plan_insert(old, 0, stride) {
            Ok(plan) => {
                prop_assert!(wide <= usize::MAX as u128);
                prop_assert_eq!(plan.total() as u128, wide);
            }
            Err(error) => {
                prop_assert_eq!(error, ArrayError::SizeOverflow);
                prop_assert!(wide > usize::MAX as u128);
            }
        }
    }

    #[test]
    fn exclude_sizes_overflow_exactly_when_128_bit_arithmetic_does(
        old in prop_oneof![any::<usize>(), (0usize..64), Just(usize::MAX)],
        stride in prop_oneof![any::<usize>(), (0usize..64), Just(usize::MAX)],
    ) {
        let wide = old as u128 * stride as u128;
        match plan_exclude(old, 0, 0, stride) {
            Ok(plan) => {
                prop_assert!(wide <= usize::MAX as u128);
                prop_assert_eq!(plan.total() as u128, wide);
            }
            Err(error) => {
                prop_assert_eq!(error, ArrayError::SizeOverflow);
                prop_assert!(wide > usize::MAX as u128);
            }
        }
    }
}

#[test]
fn positions_outside_the_array_are_errors() {
    assert_eq!(
        plan_insert(2, -1, 4),
        Err(ArrayError::Negative("insertPos"))
    );
    assert!(matches!(
        plan_insert(2, 3, 4),
        Err(ArrayError::OutOfRange(_))
    ));
    assert_eq!(
        plan_exclude(2, 0, -1, 4),
        Err(ArrayError::Negative("numExclude"))
    );
    assert!(matches!(
        plan_exclude(2, 2, 1, 4),
        Err(ArrayError::OutOfRange(_))
    ));
    assert!(plan_exclude(2, 2, 0, 4).is_ok());
}
