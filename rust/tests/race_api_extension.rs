#[test]
fn teardown_receives_final_crew_and_owned_state() {
    fn descriptor() -> uqm_rust::engine::battle::RaceDesc {
        RaceTemplate::default().descriptor(&RACE)
    }
    fn uninit(stats: &mut uqm_rust::engine::battle::views::RaceStats) {
        uqm_rust::battle::set_flagship_crew(stats.ship_info.crew_level);
        stats.state = RaceState::None;
    }
    static RACE: Race = Race {
        descriptor,
        preprocess: None,
        postprocess: None,
        init_weapon: None,
        intelligence: None,
        uninit: Some(uninit),
    };
    battle_context::run(|| {
        let owner = Battle::new(2);
        // SAFETY: the serialized fixture owns the enlisted-crew authority and restores it after teardown.
        unsafe {
            let saved_crew = GlobData.SIS_state.CrewEnlisted;
            let mut ship = Ship::native(&owner, &RACE, 0);
            uqm_rust::app::bridge::enter(|cx| {
                cx.race_stats_mut(ship.ship_id).ship_info.crew_level = 23;
                cx.race_stats_mut(ship.ship_id).state = RaceState::Umgah {
                    previous_facing: Some(4),
                };
            });
            ship.uninit();
            uqm_rust::app::bridge::enter(|cx| {
                assert_eq!(cx.flagship_config().crew, 23);
                assert!(matches!(cx.race_stats(ship.ship_id).state, RaceState::None));
            });
            drop(ship);
            GlobData.SIS_state.CrewEnlisted = saved_crew;
        }
    });
}

#[test]
fn abandonment_keeps_allocation_rng_parent_and_target_order() {
    battle_context::run(|| {
        let owner = Battle::new(4);
        // SAFETY: fixtures retain all frames and queue records; the shared frame authority is restored.
        unsafe {
            let a = Ship::new(&owner, SPECIES_ID_VUX_ID, 0);
            let b = Ship::new(&owner, SPECIES_ID_VUX_ID, 1);
            let frames = battle::FrameSet::new(4);
            let old = stars_in_space;
            stars_in_space = frames.frame(0);
            uqm_rust::app::bridge::enter(|cx| {
                let crew = cx.race_stats(a.ship_id).ship_info.crew_level;
                battle::with_rng(1234, 4, || cx.abandon_ship(a.id, b.id, 3));
                assert_eq!(cx.race_stats(a.ship_id).ship_info.crew_level, crew - 2);
                let first = cx.elements().succ(b.id).unwrap();
                let second = cx.elements().succ(first).unwrap();
                assert_eq!(cx.elements().tail(), Some(second));
                assert_eq!(cx.owner(first), a.ship_id);
                assert_eq!(cx.owner(second), b.ship_id);
                for id in [first, second] {
                    assert_eq!(cx.target(id), Some(b.id));
                    assert_eq!(cx.element(id).life, 300);
                    assert_ne!(cx.element(id).flags & CREW_OBJECT as u16, 0);
                    cx.unlink(id);
                    cx.dispose(id);
                }
            });
            stars_in_space = old;
        }
    });
}

#[test]
fn self_destruct_selects_a_surviving_opponent_before_the_dead_owner() {
    battle_context::run(|| {
        let owner = Battle::new(4);
        // SAFETY: both live queue fixtures retain their descriptors during winner selection.
        unsafe {
            let a = Ship::new(&owner, SPECIES_ID_VUX_ID, 0);
            let b = Ship::new(&owner, SPECIES_ID_VUX_ID, 1);
            uqm_rust::app::bridge::enter(|cx| {
                cx.select_self_destruct_winner(a.id);
                assert_eq!(cx.winner(), Some(b.ship_id));
                assert_ne!(
                    cx.ship_state(b.ship_id).status & PLAY_VICTORY_DITTY as u16,
                    0
                );
            });
        }
    });
}

#[test]
fn sound_banks_are_per_ship_and_pkunk_taunts_share_the_existing_authority() {
    battle_context::run(|| {
        let owner = Battle::new(4);
        let mut a = Ship::bare(&owner, SPECIES_ID_VUX_ID, 0);
        let mut b = Ship::bare(&owner, SPECIES_ID_VUX_ID, 1);
        // SAFETY: sound tables and serialized fixtures stay live until the bank pointers are cleared.
        unsafe {
            use uqm_rust::strings::string_table::*;
            let table = AllocStringTable(5, 0);
            a.descriptor_mut().ship_data.ship_sounds = CaptureStringTable(table).cast();
            b.descriptor_mut().ship_data.ship_sounds = CaptureStringTable(table).cast();
            uqm_rust::app::bridge::enter(|cx| {
                assert_eq!(cx.ship_sound_count(a.ship_id), 5);
                cx.set_ship_sound_bank(a.ship_id, 2);
                assert_eq!(cx.ship_sound_count(a.ship_id), 5);
                let saved = cx.last_pkunk_sound();
                cx.set_last_pkunk_sound(4);
                assert_eq!(cx.last_pkunk_sound(), 4);
                cx.set_last_pkunk_sound(saved);
            });
            assert_eq!(
                GetStringTableIndex(a.descriptor_mut().ship_data.ship_sounds.cast()),
                2
            );
            assert_eq!(
                GetStringTableIndex(b.descriptor_mut().ship_data.ship_sounds.cast()),
                0
            );
            a.descriptor_mut().ship_data.ship_sounds = std::ptr::null_mut();
            b.descriptor_mut().ship_data.ship_sounds = std::ptr::null_mut();
            FreeStringTable(table);
        }
    });
}

#[test]
fn sound_bank_selection_rejects_an_out_of_range_index() {
    battle_context::run(|| {
        let owner = Battle::new(2);
        let ship = Ship::bare(&owner, SPECIES_ID_VUX_ID, 0);
        // SAFETY: the fixture retains its descriptor while the checked operation rejects its absent bank.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    cx.set_ship_sound_bank(ship.ship_id, 0);
                }));
                assert!(result.is_err());
            });
        }
    });
}

#[test]
fn self_destruct_winner_preserves_simultaneous_death_and_an_existing_winner() {
    battle_context::run(|| {
        let owner = Battle::new(4);
        // SAFETY: fixtures own their queue slots and descriptors through winner selection.
        unsafe {
            let a = Ship::new(&owner, SPECIES_ID_VUX_ID, 0);
            let b = Ship::new(&owner, SPECIES_ID_VUX_ID, 1);
            uqm_rust::app::bridge::enter(|cx| {
                cx.race_stats_mut(b.ship_id).ship_info.crew_level = 0;
                assert!(cx.winner().is_none());
                cx.select_self_destruct_winner(a.id);
                assert_eq!(cx.winner(), Some(a.ship_id));
                assert_ne!(
                    cx.ship_state(a.ship_id).status & PLAY_VICTORY_DITTY as u16,
                    0
                );
                cx.select_self_destruct_winner(a.id);
                assert_eq!(
                    cx.ship_state(a.ship_id).status & PLAY_VICTORY_DITTY as u16,
                    0
                );
                cx.select_self_destruct_winner(b.id);
                assert_eq!(cx.winner(), Some(a.ship_id));
            });
        }
    });
}

#[test]
fn abandonment_pool_exhaustion_keeps_crew_and_rng_unchanged() {
    battle_context::run(|| {
        let owner = Battle::new(2);
        // SAFETY: loaded fixture frames and both ships remain live during the full-pool attempt.
        unsafe {
            let a = Ship::new(&owner, SPECIES_ID_VUX_ID, 0);
            let b = Ship::new(&owner, SPECIES_ID_VUX_ID, 1);
            uqm_rust::app::bridge::enter(|cx| {
                let crew = cx.race_stats(a.ship_id).ship_info.crew_level;
                battle::with_rng(1234, 0, || cx.abandon_ship(a.id, b.id, 2));
                assert_eq!(cx.race_stats(a.ship_id).ship_info.crew_level, crew);
                assert_eq!(cx.elements().tail(), Some(b.id));
            });
        }
    });
}

#[test]
fn navigation_world_projections_write_the_single_authority() {
    battle_context::run(|| {
        let _owner = Battle::new(2);
        // SAFETY: this test serializes and restores every modified world value.
        unsafe {
            let activity = GlobData.Game_state.CurrentActivity;
            let autopilot = GlobData.Game_state.autopilot;
            let velocity = GlobData.Game_state.velocity;
            let sis = GlobData.SIS_state;
            uqm_rust::state::ffi::rust_init_game_state();
            uqm_rust::app::bridge::enter(|cx| {
                cx.set_activity(42);
                assert_eq!(cx.world.activity(), 42);
                cx.set_autopilot(POINT { x: 17, y: 19 });
                assert_eq!((cx.autopilot().x, cx.autopilot().y), (17, 19));
                cx.set_flagship_crew(23);
                let config = cx.flagship_config();
                assert_eq!(config.crew, 23);
                assert_eq!(config.modules, sis.ModuleSlots);
                assert_eq!(config.drives, sis.DriveSlots);
                assert_eq!(config.jets, sis.JetSlots);
                assert_eq!(config.fuel, sis.FuelOnBoard);
                assert_eq!(config.position, (sis.log_x, sis.log_y));
                let mut next = cx.flagship_velocity();
                next.set_components(25, -31);
                cx.set_flagship_velocity(next);
                assert_eq!(cx.flagship_velocity().get_current_components(), (25, -31));
                let flag = cx.story_state("SHOFIXTI_RECRUITED");
                cx.set_story_state("SHOFIXTI_RECRUITED", 1);
                assert_eq!(cx.story_state("SHOFIXTI_RECRUITED"), 1);
                cx.set_story_state("SHOFIXTI_RECRUITED", flag);
                cx.set_activity(activity);
                cx.set_autopilot(autopilot);
                cx.set_flagship_velocity(velocity);
                cx.set_flagship_crew(sis.CrewEnlisted);
            });
        }
    });
}

#[test]
fn typed_laser_and_thrust_use_live_ship_geometry_and_lifecycle() {
    battle_context::run(|| {
        let owner = Battle::new(4);
        // SAFETY: fixtures retain pool slots and the serialized battle authority.
        unsafe {
            let ship = Ship::new(&owner, SPECIES_ID_VUX_ID, 0);
            uqm_rust::app::bridge::enter(|cx| {
                let _ = cx.apply_inertial_thrust(ship.id);
                assert!(!cx.element(ship.id).velocity.is_zero());
                cx.set_target(ship.id, Some(ship.id));
                cx.untarget(ship.id);
                assert_eq!(cx.target(ship.id), None);
                let id = cx
                    .spawn_laser(Laser {
                        origin: POINT { x: 123, y: 234 },
                        end: POINT { x: 40, y: 0 },
                        flags: IGNORE_SIMILAR as u16,
                        sender: 0,
                        pixoffs: 0,
                        face: 0,
                        color: Color {
                            r: 10,
                            g: 20,
                            b: 30,
                            a: 255,
                        },
                    })
                    .unwrap();
                assert_eq!(cx.element(id).life, 1);
                assert_eq!(cx.element(id).current.location.x, 123);
                assert_eq!(cx.primitive_style(id).kind, gfx_object_LINE_PRIM as u8);
                assert_eq!(cx.primitive_style(id).color.g, 20);
                cx.dispose(id);
            });
        }
    });
}

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use battle::{Battle, Ship};
use uqm_rust::engine::battle::{race::*, views::*, *};
use uqm_rust::ships::native::abi::*;

fn empty_descriptor() -> RaceDesc {
    RaceTemplate::default().descriptor(&EMPTY_RACE)
}
static EMPTY_RACE: Race = Race {
    descriptor: empty_descriptor,
    preprocess: None,
    postprocess: None,
    init_weapon: None,
    intelligence: None,
    uninit: None,
};

#[test]
fn absent_callbacks_remain_absent_in_metadata_and_dispatch() {
    battle_context::run(|| {
        let owner = Battle::new(4);
        let ship = Ship::native(&owner, &EMPTY_RACE, 0);
        assert_eq!(empty_descriptor().into_legacy().callbacks(), [0; 5]);
        // SAFETY: the serialized fixture retains all pool storage.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                assert!(cx
                    .spawn_weapons(ship.ship_id, cx.weapon_origin(ship.id))
                    .is_empty());
                let mut concerns = Concerns {
                    subject: cx.weapon_origin(ship.id),
                    objects: [Concern {
                        object: None,
                        facing: 0,
                        which_turn: 0,
                        movement: 0,
                    }; 4],
                    count: 0,
                };
                uqm_rust::collections::observation::enable();
                uqm_rust::collections::observation::drain();
                assert!(!cx.race_intelligence(ship.ship_id, &mut concerns));
                assert!(!cx.invoke_ship_process(ship.ship_id, false));
                assert!(!uqm_rust::collections::observation::drain()
                    .iter()
                    .any(|e| matches!(e, uqm_rust::collections::observation::Event::Hook { .. })));
            });
        }
    });
}

#[test]
fn frames_keep_wrapping_bounds_absence_and_ship_view_identity() {
    battle_context::run(|| {
        let owner = Battle::new(4);
        // SAFETY: fixture frames stay loaded throughout the context operations.
        unsafe {
            let ship = Ship::new(&owner, SPECIES_ID_VUX_ID, 0);
            let first = ship.frame(0, 0);
            let set = ship.frame_set(2);
            uqm_rust::app::bridge::enter(|cx| {
                assert!(!first.is_absent());
                assert!(Frame::ABSENT.is_absent());
                assert_eq!(cx.frame_count(Frame::ABSENT), 0);
                assert_eq!(cx.frame_count(first), 16);
                let last = cx.retreat_frame(first);
                assert_eq!(cx.frame_number(last), 15);
                assert_eq!(cx.advance_frame(last), first);
                assert_eq!(cx.relative_frame(first, -1), last);
                let bounds = cx.frame_rect(first);
                assert_eq!((bounds.corner.x, bounds.corner.y), (0, 0));
                assert_eq!((bounds.extent.width, bounds.extent.height), (4, 4));
                cx.set_ship_frame(ship.ship_id, 2, 0, last);
                assert_eq!(cx.ship_frame(ship.ship_id, 2), last);
                assert_eq!(cx.ship_frame_set(ship.ship_id, 2), set);
                cx.set_captain_special(ship.ship_id, last);
                assert_eq!(cx.captain_special(ship.ship_id), last);
            });
        }
    });
}

#[test]
fn allocated_copy_keeps_primitive_identity_parent_target_and_observations() {
    battle_context::run(|| {
        let owner = Battle::new(4);
        // SAFETY: the fixtures and destination slot belong to this serialized battle.
        unsafe {
            let ship = Ship::new(&owner, SPECIES_ID_VUX_ID, 0);
            uqm_rust::app::bridge::enter(|cx| {
                let dest = cx.alloc_element().unwrap();
                let index = cx.elements()[dest].PrimIndex;
                cx.set_target(ship.id, Some(ship.id));
                cx.primitive_style_mut(ship.id).color = Color {
                    r: 1,
                    g: 2,
                    b: 3,
                    a: 255,
                };
                uqm_rust::collections::observation::enable();
                uqm_rust::collections::observation::drain();
                cx.copy_allocated_element(dest, ship.id);
                assert_eq!(cx.elements()[dest].PrimIndex, index);
                assert_ne!(index, cx.elements()[ship.id].PrimIndex);
                assert_eq!(cx.owner(dest), ship.ship_id);
                assert_eq!(cx.target(dest), Some(ship.id));
                assert_eq!(
                    cx.element(dest).current.image.frame,
                    cx.element(ship.id).current.image.frame
                );
                assert_eq!(cx.primitive_style(dest).kind, gfx_object_STAMP_PRIM as u8);
                assert_eq!(cx.primitive_style(dest).color.g, 2);
                let events = uqm_rust::collections::observation::drain();
                assert!(
                    matches!(events.as_slice(), [uqm_rust::collections::observation::Event::ImageCopy { target, source, bytes }]
                    if *target == cx.elements().raw(dest) as usize
                        && *source == cx.elements().raw(ship.id) as usize
                        && *bytes == std::mem::size_of::<ELEMENT>())
                );
                cx.put(dest);
                cx.unlink(dest);
                cx.dispose(dest);
                assert!(cx.elements().linked(ship.id));
            });
        }
    });
}

#[test]
fn per_ship_state_and_control_round_trip_without_sharing() {
    battle_context::run(|| {
        let owner = Battle::new(4);
        let a = Ship::bare(&owner, SPECIES_ID_VUX_ID, 0);
        let b = Ship::bare(&owner, SPECIES_ID_VUX_ID, 1);
        // SAFETY: checked fixtures retain both descriptor copies.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                let alternate = cx.race_stats(a.ship_id).characteristics;
                cx.race_stats_mut(a.ship_id).state = RaceState::Mmrnmhrm { alternate };
                assert!(matches!(cx.race_stats(b.ship_id).state, RaceState::None));
                cx.race_stats_mut(a.ship_id).state = RaceState::Pkunk {
                    phoenix: Some(b.id),
                    saved_pre: None,
                    saved_post: None,
                    saved_death: None,
                };
                assert!(
                    matches!(cx.race_stats(a.ship_id).state, RaceState::Pkunk { phoenix: Some(id), .. } if id == b.id)
                );
                cx.ship_state_mut(a.ship_id).control = 16;
                assert_eq!(cx.ship_state(a.ship_id).control, 16);
                assert_eq!(cx.ship_state(b.ship_id).control, 0);
                cx.race_stats_mut(a.ship_id).state = RaceState::Umgah {
                    previous_facing: Some(7),
                };
                assert!(matches!(
                    cx.race_stats(a.ship_id).state,
                    RaceState::Umgah {
                        previous_facing: Some(7)
                    }
                ));
                cx.race_stats_mut(a.ship_id).state = RaceState::Sis(SisState::default());
                cx.race_stats_mut(a.ship_id).state = RaceState::SaMatra(SaMatraState::default());
                let RaceState::SaMatra(mut boss) = cx.race_stats(a.ship_id).state else {
                    panic!("boss state")
                };
                boss.generators[0] = Some(b.id);
                boss.destroyed = 1;
                cx.race_stats_mut(a.ship_id).state = RaceState::SaMatra(boss);
                assert!(
                    matches!(cx.race_stats(a.ship_id).state, RaceState::SaMatra(s) if s.generators[0] == Some(b.id) && s.destroyed == 1)
                );
            });
        }
    });
}

#[test]
fn template_preserves_typed_resources_stats_and_initial_state() {
    let mut template = RaceTemplate::default();
    template.info.crew_level = 17;
    template.characteristics.max_thrust = 23;
    template.resources.ship[0] = Some(c"ship.synthetic.graphics");
    template.resources.captain = Some(c"ship.synthetic.captain");
    template.state = RaceState::Umgah {
        previous_facing: None,
    };
    let raw = template.descriptor(&EMPTY_RACE).into_legacy();
    assert_eq!(raw.ship_info.crew_level, 17);
    assert_eq!(raw.characteristics.max_thrust, 23);
    assert_eq!(
        raw.ship_data.ship_rsc[0],
        c"ship.synthetic.graphics".as_ptr()
    );
    assert!(raw.ship_data.ship_rsc[1].is_null());
    assert_eq!(
        raw.ship_data.captain_control.captain_rsc,
        c"ship.synthetic.captain".as_ptr()
    );
    assert!(matches!(
        raw.race_state,
        RaceState::Umgah {
            previous_facing: None
        }
    ));
}
