use uqm_engine::collections::pool::OrderedPool;

#[test]
fn paired_borrows_preserve_argument_order_and_mutate_the_owned_slots() {
    let mut pool = OrderedPool::<u8, 2>::new();
    let a = pool.alloc(1).unwrap();
    let b = pool.alloc(2).unwrap();
    let (right, left) = pool.get_pair_mut(b, a);
    *right += 10;
    *left += 20;
    assert_eq!((*pool.get(a), *pool.get(b)), (21, 12));
    let (left, right) = pool.get_pair_mut(a, b);
    *left += 1;
    *right += 1;
    assert_eq!((*pool.get(a), *pool.get(b)), (22, 13));
}

#[test]
#[should_panic(expected = "distinct pool slots")]
fn paired_borrows_reject_aliasing() {
    let mut pool = OrderedPool::<u8, 1>::new();
    let a = pool.alloc(1).unwrap();
    pool.get_pair_mut(a, a);
}

#[test]
#[should_panic(expected = "stale pool id")]
fn paired_borrows_reject_retired_generations() {
    let mut pool = OrderedPool::<u8, 2>::new();
    let a = pool.alloc(1).unwrap();
    let b = pool.alloc(2).unwrap();
    pool.free(a);
    pool.alloc(3).unwrap();
    pool.get_pair_mut(a, b);
}
