use super::*;
use crate::battle::element::Element;
use crate::collections::observation::{self, image, Event};

fn observed_set<T>(views: &[*mut T]) -> crate::engine::battle::frames::Frames {
    use crate::engine::battle::{
        frames::{FrameArray, Frames, SharedFrames},
        views::Frame,
    };
    Frames::shared(
        SharedFrames::Fixture(0),
        FrameArray::with_source(
            std::array::from_fn(|index| {
                // SAFETY: journal fixtures use these retained handles only as opaque identities.
                unsafe { Frame::from_native(views[index.min(views.len() - 1)].cast()) }
            }),
            (),
            Some(views.as_ptr() as usize),
        ),
    )
}

fn fields(journal: &mut Journal, element: &Element) -> Fields {
    let identity = journal.identity(std::ptr::from_ref(element) as usize);
    let mut fields = Fields::new();
    for (name, frame) in [
        ("current.frame", element.current.frame),
        ("next.frame", element.next.frame),
    ] {
        journal.write_element_frame(&identity, name, frame as usize, &mut fields, name);
    }
    for (name, array) in [
        ("current.frames", &element.current.frames),
        ("next.frames", &element.next.frames),
    ] {
        journal.write_element_frame_set(&identity, name, array.raw() as usize, &mut fields, name);
    }
    fields
}

fn drawable(owner: usize, frame: usize, index: u16) {
    observation::record(Event::Drawable(owner));
    observation::record(Event::Frame {
        address: frame,
        owner,
        index,
    });
}

#[test]
#[serial_test::serial]
fn hyperspace_swaps_refresh_the_live_global_view_and_restore_its_owner() {
    observation::enable();
    observation::drain();
    let directory = crate::automation::parity_state::test_directory();
    let mut journal = Journal::create(directory.path()).expect("journal");
    let mut element = Element::new();
    let mut stars = 200usize as *mut std::ffi::c_void;
    let mut ambient = 400usize as *mut std::ffi::c_void;
    observation::record(Event::Allocate(std::ptr::from_ref(&element) as usize));
    drawable(100, stars as usize, 7);
    drawable(300, ambient as usize, 14);
    observation::observe_views(std::slice::from_ref(&stars), "engine-global:stars", |p| {
        *p as usize
    });
    image!(single; element.current.frames, observed_set(std::slice::from_ref(&stars)), stars);
    journal.drain_events(&[]);
    let before = fields(&mut journal, &element);
    observation::swap_image_view(&mut ambient, &mut stars);
    journal.drain_events(&[]);
    let inside = fields(&mut journal, &element);
    assert_eq!(ambient as usize, 200);
    assert_eq!(stars as usize, 400);
    assert_eq!(inside["current.frames.0.index"], "14");
    assert_ne!(
        inside["current.frames.0.owner"],
        before["current.frames.0.owner"]
    );
    observation::swap_image_view(&mut ambient, &mut stars);
    journal.drain_events(&[]);
    assert_eq!(ambient as usize, 400);
    assert_eq!(stars as usize, 200);
    assert_eq!(fields(&mut journal, &element), before);
}

#[test]
#[serial_test::serial]
fn same_generation_rebind_and_stale_reference_have_different_provenance() {
    assert_eq!(rebind_history(false), rebind_history(true));
}

fn rebind_history(reuse: bool) -> Fields {
    observation::enable();
    observation::drain();
    let directory = crate::automation::parity_state::test_directory();
    let mut journal =
        Journal::create(directory.path()).unwrap_or_else(|error| panic!("journal: {error}"));
    let mut element = Element::new();
    let pointer = 200usize as *mut std::ffi::c_void;
    let mut views = [pointer; 3];
    let array = views.as_mut_ptr();
    observation::record(Event::Allocate(std::ptr::from_ref(&element) as usize));
    drawable(100, pointer as usize, 7);
    observation::record(Event::FrameSet {
        address: array as usize,
        frames: vec![pointer as usize; 3],
        role: "old-set".into(),
    });
    image!(frame; element.current.frame, pointer);
    image!(set; element.current.frames, observed_set(&views));
    journal.drain_events(&[]);
    let old = fields(&mut journal, &element);
    let identity = journal.identity(std::ptr::from_ref(&element) as usize);
    observation::record(Event::DrawableFree(100));
    let replacement = if reuse {
        pointer
    } else {
        300usize as *mut std::ffi::c_void
    };
    drawable(100, replacement as usize, 99);
    views.fill(replacement);
    observation::record(Event::FrameSet {
        address: array as usize,
        frames: vec![replacement as usize; 3],
        role: "new-set".into(),
    });
    journal.drain_events(&[]);
    let stale = fields(&mut journal, &element);
    assert_eq!(stale["current.frame.owner"], old["current.frame.owner"]);
    assert_eq!(stale["current.frame.retired"], "true");
    assert_eq!(
        stale["current.frames.2.owner"],
        old["current.frames.2.owner"]
    );
    image!(copy; element.next, element.current);
    image!(frame; element.current.frame, replacement);
    image!(set; element.current.frames, observed_set(&views));
    journal.drain_events(&[]);
    let rebound = fields(&mut journal, &element);
    assert_eq!(
        identity,
        journal.identity(std::ptr::from_ref(&element) as usize)
    );
    assert_ne!(rebound["current.frame.owner"], old["current.frame.owner"]);
    assert_eq!(rebound["current.frame.index"], "99");
    assert_eq!(rebound["current.frame.retired"], "false");
    assert_eq!(rebound["current.frames.role"], "new-set");
    assert_ne!(
        rebound["current.frames.2.owner"],
        old["current.frames.2.owner"]
    );
    assert_eq!(rebound["next.frame.owner"], old["current.frame.owner"]);
    assert_eq!(rebound["next.frame.retired"], "true");
    assert_eq!(
        rebound["next.frames.2.owner"],
        old["current.frames.2.owner"]
    );
    assert_eq!(rebound["next.frames.role"], "old-set");
    rebound
}

fn frame_set(views: &[*mut std::ffi::c_void], role: &str) {
    observation::record(Event::FrameSet {
        address: views.as_ptr() as usize,
        frames: views.iter().map(|frame| *frame as usize).collect(),
        role: role.into(),
    });
}

#[test]
#[serial_test::serial]
fn unsampled_assignments_and_individual_copies_survive_reuse() {
    observation::enable();
    observation::drain();
    let directory = crate::automation::parity_state::test_directory();
    let mut journal =
        Journal::create(directory.path()).unwrap_or_else(|error| panic!("journal: {error}"));
    let mut element = Element::new();
    let pointer = 200usize as *mut std::ffi::c_void;
    let views = [pointer; 3];
    observation::record(Event::Allocate(std::ptr::from_ref(&element) as usize));
    drawable(100, pointer as usize, 7);
    frame_set(&views, "old-set");
    image!(frame; element.current.frame, pointer);
    image!(set; element.current.frames, observed_set(&views));
    observation::record(Event::DrawableFree(100));
    drawable(100, pointer as usize, 99);
    frame_set(&views, "new-set");
    image!(copy; element.next.frame, element.current.frame);
    image!(copy; element.next.frames, element.current.frames);
    image!(frame; element.current.frame, pointer);
    image!(set; element.current.frames, observed_set(&views));
    journal.drain_events(&[]);
    let observed = fields(&mut journal, &element);
    assert_eq!(observed["next.frame.index"], "7");
    assert_eq!(observed["next.frame.retired"], "true");
    assert_eq!(observed["next.frames.2.index"], "7");
    assert_eq!(observed["next.frames.role"], "old-set");
    assert_eq!(observed["current.frame.index"], "99");
    assert_eq!(observed["current.frames.2.index"], "99");
    assert_eq!(observed["current.frames.role"], "new-set");
    assert_ne!(
        observed["current.frame.owner"],
        observed["next.frame.owner"]
    );
}

#[test]
#[serial_test::serial]
fn array_edits_update_only_references_to_the_written_resource_generation() {
    observation::enable();
    observation::drain();
    let directory = crate::automation::parity_state::test_directory();
    let mut journal =
        Journal::create(directory.path()).unwrap_or_else(|error| panic!("journal: {error}"));
    let mut element = Element::new();
    let pointer = 200usize as *mut std::ffi::c_void;
    let mut views = [pointer; 3];
    observation::record(Event::Allocate(std::ptr::from_ref(&element) as usize));
    drawable(100, pointer as usize, 7);
    frame_set(&views, "old-set");
    observation::record(Event::ResetPool);
    observation::record(Event::Allocate(std::ptr::from_ref(&element) as usize));
    image!(set; element.current.frames, observed_set(&views));
    observation::record(Event::Frame {
        address: 201,
        owner: 100,
        index: 8,
    });
    image!(view; views[0], 201usize as *mut std::ffi::c_void);
    image!(copy; element.next, element.current);
    journal.drain_events(&[]);
    let before = fields(&mut journal, &element);
    assert_eq!(before["current.frames.0.index"], "8");
    assert_eq!(before["next.frames.0.index"], "8");
    observation::record(Event::DrawableFree(100));
    drawable(100, pointer as usize, 99);
    views.fill(pointer);
    frame_set(&views, "new-set");
    image!(set; element.current.frames, observed_set(&views));
    observation::record(Event::Frame {
        address: 201,
        owner: 100,
        index: 100,
    });
    image!(view; views[0], 201usize as *mut std::ffi::c_void);
    journal.drain_events(&[]);
    let after = fields(&mut journal, &element);
    assert_eq!(after["current.frames.0.index"], "100");
    assert_eq!(after["next.frames.0.index"], "8");
    assert_eq!(after["next.frames.0.retired"], "true");
    assert_eq!(after["next.frames.2.owner"], before["next.frames.2.owner"]);
    assert_ne!(
        after["current.frames.2.owner"],
        after["next.frames.2.owner"]
    );
}

#[test]
#[serial_test::serial]
fn raw_abi_image_and_state_copies_keep_the_source_identity() {
    use crate::ships::native::abi::{state, state__bindgen_ty_1, POINT};
    observation::enable();
    observation::drain();
    let directory = crate::automation::parity_state::test_directory();
    let mut journal =
        Journal::create(directory.path()).unwrap_or_else(|error| panic!("journal: {error}"));
    let mut current = state {
        location: POINT { x: 7, y: 9 },
        image: state__bindgen_ty_1 {
            frame: std::ptr::null_mut(),
            frames: crate::engine::battle::views::Frames::ABSENT,
        },
    };
    let mut next = current.clone();
    let pointer = 200usize as crate::ships::native::abi::FRAME;
    let views = [pointer; 3];
    drawable(100, pointer as usize, 7);
    observation::record(Event::FrameSet {
        address: views.as_ptr() as usize,
        frames: vec![pointer as usize; 3],
        role: "old-set".into(),
    });
    image!(frame; current.image.frame, pointer);
    image!(set; current.image.frames, observed_set(&views));
    observation::record(Event::DrawableFree(100));
    drawable(100, pointer as usize, 99);
    image!(copy; next.image, current.image);
    image!(frame; current.image.frame, pointer);
    journal.drain_events(&[]);
    let old = &journal.bindings.frames[&(std::ptr::from_ref(&next.image.frame) as usize)].frame;
    let new = &journal.bindings.frames[&(std::ptr::from_ref(&current.image.frame) as usize)].frame;
    assert_eq!(old.1, 7);
    assert_eq!(new.1, 99);
    assert_ne!(old.0, new.0);
    image!(copy; current, next);
    journal.drain_events(&[]);
    assert_eq!(current.location.x, 7);
    assert_eq!(current.location.y, 9);
    assert_eq!(
        journal.bindings.frames[&(std::ptr::from_ref(&current.image.frame) as usize)]
            .frame
            .1,
        7
    );
    assert_eq!(
        journal.bindings.sets[&(std::ptr::from_ref(&current.image.frames) as usize)].frames[2].1,
        7
    );
}

#[test]
#[serial_test::serial]
fn element_field_assignment_is_not_an_array_edit_at_reused_storage() {
    observation::enable();
    observation::drain();
    let directory = crate::automation::parity_state::test_directory();
    let mut journal =
        Journal::create(directory.path()).unwrap_or_else(|error| panic!("journal: {error}"));
    let mut element = Element::new();
    let slot = std::ptr::from_ref(&element.current.frame) as usize;
    let pointer = 200usize as *mut std::ffi::c_void;
    drawable(100, pointer as usize, 7);
    observation::record(Event::FrameSet {
        address: slot,
        frames: vec![pointer as usize],
        role: "former-array-storage".into(),
    });
    observation::record(Event::DrawableFree(100));
    observation::record(Event::Allocate(std::ptr::from_ref(&element) as usize));
    drawable(100, pointer as usize, 99);
    image!(frame; element.current.frame, pointer);
    journal.drain_events(&[]);
    let observed = fields(&mut journal, &element);
    assert_eq!(observed["current.frame.index"], "99");
    assert_eq!(observed["current.frame.retired"], "false");
    assert_eq!(journal.bindings.sources[&slot].frames[0].1, 7);
    assert_ne!(
        journal.bindings.sources[&slot].frames[0].0,
        observed["current.frame.owner"]
    );
}

#[test]
#[serial_test::serial]
fn unsampled_whole_element_clones_keep_all_image_owners_after_storage_reuse() {
    assert_eq!(whole_element_clone(false), whole_element_clone(true));
}

fn whole_element_clone(reuse: bool) -> Fields {
    observation::enable();
    observation::drain();
    let directory = crate::automation::parity_state::test_directory();
    let mut journal = Journal::create(directory.path()).expect("journal");
    let mut source = Element::new();
    let mut cloned = Element::new();
    let pointer = 200usize as *mut std::ffi::c_void;
    let mut views = [pointer; 3];
    observation::record(Event::Allocate(std::ptr::from_ref(&source) as usize));
    drawable(100, pointer as usize, 7);
    observation::observe_views(&views, "old-clone-set", |p| *p as usize);
    image!(frame; source.current.frame, pointer);
    image!(set; source.current.frames, observed_set(&views));
    image!(copy; source.next, source.current);
    image!(frame; source.intersect_control.intersect_stamp.frame, pointer);
    observation::record(Event::Allocate(std::ptr::from_ref(&cloned) as usize));
    image!(copy; cloned, source);
    journal.drain_events(&[]);
    let generation = journal.identity(std::ptr::from_ref(&cloned) as usize);
    observation::record(Event::DrawableFree(100));
    let replacement = if reuse {
        pointer
    } else {
        300usize as *mut std::ffi::c_void
    };
    drawable(100, replacement as usize, 99);
    views.fill(replacement);
    observation::observe_views(&views, "replacement-clone-set", |p| *p as usize);
    journal.drain_events(&[]);
    let mut observed = fields(&mut journal, &cloned);
    journal.write_element_frame(
        &generation,
        "intersect.frame",
        cloned.intersect_control.intersect_stamp.frame as usize,
        &mut observed,
        "intersect",
    );
    for name in [
        "current.frame",
        "next.frame",
        "intersect",
        "current.frames.0",
        "current.frames.1",
        "current.frames.2",
        "next.frames.0",
        "next.frames.1",
        "next.frames.2",
    ] {
        assert_eq!(observed[&format!("{name}.index")], "7");
        assert_eq!(observed[&format!("{name}.retired")], "true");
    }
    assert_eq!(observed["current.frames.role"], "old-clone-set");
    assert_eq!(observed["next.frames.role"], "old-clone-set");
    assert_eq!(
        generation,
        journal.identity(std::ptr::from_ref(&cloned) as usize)
    );
    observed
}
