use super::*;
use std::borrow::Cow;

const STAR: CStarDesc = CStarDesc {
    star_pt: crate::graphics::native_abi::POINT { x: 100, y: 200 },
    star_type: 0,
    index: 0,
    prefix: 0,
    postfix: 0,
};
static CATALOG: [CStarDesc; CATALOG_LEN] = [STAR; CATALOG_LEN];

#[test]
fn production_catalog_is_borrowed_without_copying() {
    let mut world = World::new();
    assert!(world.selected_star().is_none());
    world.set_catalog(Cow::Borrowed(&CATALOG));
    assert!(std::ptr::eq(world.catalog(), &CATALOG));
    world.current_star = Some(StarId(7));
    assert!(std::ptr::eq(world.selected_star().unwrap(), &CATALOG[7]));
    assert_eq!(world.identify_star(&CATALOG[7]), StarId(7));
}

#[test]
fn independent_worlds_retain_their_own_immutable_catalogs() {
    let mut first = World::new();
    let mut second = World::new();
    let mut custom = CATALOG;
    custom[7].star_pt.x = 999;
    first.set_catalog(Cow::Borrowed(&CATALOG));
    second.set_catalog(Cow::Owned(custom));
    first.current_star = Some(StarId(7));
    second.current_star = Some(StarId(7));
    assert_eq!(first.selected_star().unwrap().star_pt.x, 100);
    assert_eq!(second.selected_star().unwrap().star_pt.x, 999);
    assert_eq!(first.selected_star().unwrap().star_pt.x, 100);
    second.set_catalog(Cow::Borrowed(&CATALOG));
    assert!(second.selected_star().is_none());
    assert!(first.selected_star().is_some());
}

#[test]
fn star_identity_rejects_a_record_from_another_catalog() {
    let mut world = World::new();
    world.set_catalog(Cow::Owned(CATALOG));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.identify_star(&CATALOG[0]);
    }))
    .is_err());
}

#[test]
fn a_retained_catalog_outlives_world_replacement() {
    let mut world = World::new();
    let mut custom = CATALOG;
    custom[7].star_pt.x = 999;
    world.set_catalog(Cow::Owned(custom));
    let retained = world.retain_catalog();
    world.current_star = Some(StarId(7));
    world.set_catalog(Cow::Borrowed(&CATALOG));
    assert_eq!(retained[7].star_pt.x, 999);
    assert_eq!(world.catalog()[7].star_pt.x, 100);
    assert!(world.selected_star().is_none());
}
