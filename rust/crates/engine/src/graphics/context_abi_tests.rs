use super::*;

fn allocated_context() -> *mut context_desc {
    // SAFETY: the serial context tests exclusively own context allocation and destruction.
    unsafe {
        #[cfg(feature = "debug-process")]
        {
            CreateContextAux(c"lease-test".as_ptr())
        }
        #[cfg(not(feature = "debug-process"))]
        {
            CreateContextAux()
        }
    }
}

#[test]
#[serial_test::serial]
fn filled_stamp_can_restore_color_without_changing_the_existing_setting_variant() {
    let raw = allocated_context();
    let context = ContextLease::retain(std::ptr::NonNull::new(raw).unwrap());
    let source = ContextFrame::retain(None);
    let original = Color {
        r: 17,
        g: 31,
        b: 47,
        a: 255,
    };
    let fill = Color {
        r: 91,
        g: 73,
        b: 55,
        a: 255,
    };
    context.select();
    // SAFETY: this serial fixture owns the context; its absent target skips raster drawing.
    unsafe {
        SetContextForeGroundColor(original);
        context.draw_shadowed_box(
            RECT {
                corner: POINT { x: 10, y: 20 },
                extent: EXTENT {
                    width: 30,
                    height: 40,
                },
            },
            original,
            fill,
            fill,
        );
        assert_eq!(GetContextForeGroundColor().r, original.r);
        context.draw_filled_stamp_preserving_color(&source, POINT { x: 0, y: 0 }, fill);
        assert_eq!(GetContextForeGroundColor().r, original.r);
        assert_eq!(GetContextForeGroundColor().g, original.g);
        assert_eq!(GetContextForeGroundColor().b, original.b);
        context.draw_filled_stamp(&source, POINT { x: 0, y: 0 }, fill);
        assert_eq!(GetContextForeGroundColor().r, fill.r);
        assert_eq!(GetContextForeGroundColor().g, fill.g);
        assert_eq!(GetContextForeGroundColor().b, fill.b);
        assert_eq!(DestroyContext(raw), 1);
    }
}

#[test]
#[serial_test::serial]
fn retained_context_survives_native_owner_release() {
    let raw = allocated_context();
    let context = ContextLease::retain(std::ptr::NonNull::new(raw).unwrap());
    // SAFETY: this consumes the allocation's native owner; the typed lease remains live.
    assert_eq!(unsafe { DestroyContext(raw) }, 1);
    context.set_background(Color {
        r: 17,
        g: 31,
        b: 47,
        a: 255,
    });
    // SAFETY: context still owns the registered allocation.
    assert_eq!(unsafe { (*raw).BackGroundColor.r }, 17);
    drop(context);
    // SAFETY: the serial test owns the native context state.
    assert!(unsafe { _pCurContext }.is_null());
}

#[test]
#[serial_test::serial]
fn retained_batch_restores_the_previous_context_and_depth_after_unwinding() {
    let previous_raw = allocated_context();
    let current_raw = allocated_context();
    let previous = ContextLease::retain(
        std::ptr::NonNull::new(previous_raw)
            .unwrap_or_else(|| panic!("fixture context allocation failed")),
    );
    let current = ContextLease::retain(
        std::ptr::NonNull::new(current_raw)
            .unwrap_or_else(|| panic!("fixture context allocation failed")),
    );
    previous.select();
    // SAFETY: this serial test exclusively owns native context and batch state.
    let state = || unsafe {
        (
            std::ptr::addr_of!(_pCurContext).read(),
            super::super::gfx_common_abi::get_batch_depth(),
        )
    };
    fn scope<R>(context: &ContextLease, batch: bool, draw: impl FnOnce() -> R) -> R {
        if batch {
            context.draw_batch(draw)
        } else {
            context.with_selected(draw)
        }
    }
    let depth = state().1;
    for batch in [true, false] {
        previous.select();
        let increment = if batch { 1 } else { 0 };
        assert_eq!(
            scope(&current, batch, || {
                assert_eq!(state(), (current_raw, depth + increment));
                scope(&previous, batch, || {
                    assert_eq!(state(), (previous_raw, depth + 2 * increment));
                });
                assert_eq!(state(), (current_raw, depth + increment));
                if batch {
                    {
                        let _pause = current.pause_batch();
                        assert_eq!(state(), (current_raw, depth));
                    }
                    assert_eq!(state(), (current_raw, depth + 1));
                    {
                        let _pause = previous.pause_batch();
                        assert_eq!(state(), (current_raw, depth));
                    }
                    assert_eq!(state(), (current_raw, depth + 1));
                    let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let _pause = current.pause_batch();
                        panic!("paused drawing failure");
                    }));
                    assert!(rejected.is_err());
                    assert_eq!(state(), (current_raw, depth + 1));
                }
                17
            }),
            17
        );
        assert_eq!(state(), (previous_raw, depth));
        let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scope(&current, batch, || panic!("fixture drawing failure"));
        }));
        assert!(rejected.is_err());
        assert_eq!(state(), (previous_raw, depth));
        // SAFETY: both fixture contexts remain owned while clearing the selection.
        unsafe { SetContext(std::ptr::null_mut()) };
        assert_eq!(scope(&current, batch, || 21), 21);
        assert_eq!(state(), (std::ptr::null_mut(), depth));
    }
    // SAFETY: the serial test consumes both fixture allocation owners exactly once.
    unsafe {
        assert_eq!(DestroyContext(current_raw), 1);
        assert_eq!(DestroyContext(previous_raw), 1);
    }
    drop(current);
    drop(previous);
}

#[test]
#[serial_test::serial]
fn scoped_clip_and_origin_restore_after_unwind_and_target_changes() {
    let raw = allocated_context();
    let context = ContextLease::retain(std::ptr::NonNull::new(raw).unwrap());
    // SAFETY: the serial fixture owns this new two-frame drawable and its context.
    let drawable = unsafe { super::super::drawable_abi::AllocDrawable(2) };
    // SAFETY: both frames belong to this initialized allocation.
    let (first, second) = unsafe {
        let first = super::super::pixmap_abi::CaptureDrawable(drawable);
        (
            first,
            super::super::drawable_abi::retain_frame_allocations(&[first])[0]
                .frame(1)
                .unwrap(),
        )
    };
    let first = ContextFrame::retain(std::ptr::NonNull::new(first));
    let second = ContextFrame::retain(std::ptr::NonNull::new(second));
    first.replace_hotspot(POINT { x: -3, y: 7 });
    second.replace_hotspot(POINT { x: 9, y: 8 });
    context.set_frame(first.clone());
    context.clear_clip();
    let clip = RECT {
        corner: POINT { x: 1, y: 2 },
        extent: EXTENT {
            width: 3,
            height: 4,
        },
    };
    let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        context.with_clip(clip, || {
            assert_eq!(context.clip_rect().extent.width, 3);
            context.with_origin(POINT { x: 4, y: 5 }, || {
                assert_eq!((first.rect().corner.x, first.rect().corner.y), (-4, -5));
                context.set_frame(second.clone());
                panic!("fixture drawing failure");
            });
        });
    }));
    assert!(rejected.is_err());
    assert_eq!((first.rect().corner.x, first.rect().corner.y), (3, -7));
    assert_eq!((second.rect().corner.x, second.rect().corner.y), (-9, -8));
    // SAFETY: inspect the retained context, then release only the fixture's native owners.
    unsafe {
        assert_eq!((*raw).ClipRect.extent.width, 0);
        assert_eq!(super::super::drawable_abi::DestroyDrawable(drawable), 1);
        assert_eq!(DestroyContext(raw), 1);
    }
}

#[test]
#[serial_test::serial]
fn context_retention_rejects_unowned_and_retired_addresses() {
    assert!(
        std::panic::catch_unwind(|| ContextLease::retain(std::ptr::NonNull::dangling())).is_err()
    );
    let raw = allocated_context();
    // SAFETY: the native owner is consumed without any retained lease.
    assert_eq!(unsafe { DestroyContext(raw) }, 1);
    assert!(std::panic::catch_unwind(|| ContextLease::retain(
        std::ptr::NonNull::new(raw).unwrap()
    ))
    .is_err());
}

#[test]
#[serial_test::serial]
fn a_context_target_retains_its_frame_after_native_drawable_release() {
    let raw = allocated_context();
    let context = ContextLease::retain(std::ptr::NonNull::new(raw).unwrap());
    // SAFETY: the serial test exclusively owns the newly allocated drawable and context.
    let drawable = unsafe { super::super::drawable_abi::AllocDrawable(1) };
    // SAFETY: AllocDrawable initialized the drawable's first frame.
    let frame = unsafe { super::super::pixmap_abi::CaptureDrawable(drawable) };
    context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(frame)));
    assert_eq!(
        // SAFETY: the context retains the allocation after its native owner is retired.
        unsafe { super::super::drawable_abi::DestroyDrawable(drawable) },
        1
    );
    context.set_clip(RECT {
        corner: POINT { x: 1, y: 2 },
        extent: EXTENT {
            width: 3,
            height: 4,
        },
    });
    let source = ContextFrame::retain(std::ptr::NonNull::new(frame));
    context.draw_stamp(&source, POINT { x: 7, y: 11 });
    // SAFETY: the context and its target remain retained by the lease.
    assert_eq!(unsafe { (*raw).ClipRect.extent.width }, 3);
    // SAFETY: consume the native context owner before releasing its retained lease.
    assert_eq!(unsafe { DestroyContext(raw) }, 1);
    drop(context);
    assert_eq!(
        super::super::drawable_abi::retain_frame_allocations(&[frame])[0].frame(0),
        Some(frame)
    );
    drop(source);
    assert!(std::panic::catch_unwind(|| {
        super::super::drawable_abi::retain_frame_allocations(&[frame])
    })
    .is_err());
}

#[test]
fn full_frame_clip_is_undefined_and_partial_clip_is_preserved() {
    // SAFETY: the local descriptor's scalars, flags and nullable pointers accept zero values.
    let mut context: context_desc = unsafe { std::mem::zeroed() };
    let full = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: 7,
            height: 5,
        },
    };
    let partial = RECT {
        corner: POINT { x: -2, y: 1 },
        extent: EXTENT {
            width: 4,
            height: 3,
        },
    };
    set_clip(&mut context, Some(partial), full);
    assert_eq!(
        (context.ClipRect.corner.x, context.ClipRect.extent.width),
        (-2, 4)
    );
    set_clip(&mut context, Some(full), full);
    assert_eq!(context.ClipRect.extent.width, 0);
    assert_eq!(context.ClipRect.corner.x, -2);
    set_clip(&mut context, Some(partial), full);
    set_clip(&mut context, None, full);
    assert_eq!(context.ClipRect.extent.width, 0);
}
