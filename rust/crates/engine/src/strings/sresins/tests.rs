use super::*;
use crate::strings::owned_table::LoadedBinaryTable;

#[test]
fn binary_resource_release_keeps_a_retained_owner_alive() {
    let owner = LoadedBinaryTable::from_entries(vec![vec![1, 0, 4]]);
    let detached = owner.clone().into_raw();
    // SAFETY: the resource handler receives the strong reference transferred by the loader.
    assert_eq!(unsafe { free_string_table(detached.cast()) }, 1);
    assert_eq!(owner.entry(0), Some([1, 0, 4].as_slice()));
    // SAFETY: the resource release callback also accepts an absent allocation.
    assert_eq!(unsafe { free_string_table(std::ptr::null_mut()) }, 1);
}
