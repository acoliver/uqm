#![cfg(feature = "debug-process")]

use std::{borrow::Cow, io};
use uqm_rust::{
    engine::{
        battle::{Battle, Cx},
        observe::{Io, Unobserved},
        world::{CStarDesc, NUM_SOLAR_SYSTEMS},
        Game, StarId,
    },
    front_end::uqmdebug::{dump_universe, tally_resources},
    planets::solarsys::site::dispatch::DebugGeneration,
    strings::owned_table::LoadedTextTable,
};

struct Broken;
impl io::Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
#[serial_test::serial]
fn full_diagnostics_use_the_live_site_and_restore_the_unpublished_campaign() {
    let mut game = Game::new();
    uqm_rust::state::ffi::rust_init_game_state(&mut game.world);
    game.kernel
        .install_game_text(LoadedTextTable::fixture(vec![b"Name\0".to_vec(); 600]));
    let mut catalog = uqm_rust::planets::tables::STARMAP_ARRAY;
    catalog[..NUM_SOLAR_SYSTEMS].fill(CStarDesc {
        star_pt: uqm_rust::graphics::native_abi::POINT { x: 1000, y: 2000 },
        star_type: 0,
        index: 0,
        prefix: 0,
        postfix: 0,
    });
    game.world.set_catalog(Cow::Owned(catalog));
    game.world.current_star = Some(StarId(7));
    game.world.set_activity(0x1234);
    game.world.data.game.element_worth = [1, 2, 4, 8, 16, 32, 64, 128];
    let mut battle = Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
        panic!("diagnostics must not consume the campaign RNG")
    });
    let mut observer = Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let mut generation = DebugGeneration;
    let mut dump = Vec::new();
    dump_universe(&mut cx, &mut io, &mut generation, &mut dump).unwrap();
    let dump = String::from_utf8(dump).unwrap();
    let header = format!("{:<22}  (100.0, 200.0) {:<19}  \n", "Name", "blue dwarf");
    assert!(dump.starts_with(&header));
    let systems: Vec<_> = dump.split(&header).skip(1).collect();
    assert_eq!(systems.len(), NUM_SOLAR_SYSTEMS);
    assert!(!systems[0].is_empty());
    assert!(systems.iter().all(|system| *system == systems[0]));
    assert!(systems[0].starts_with("- Name"));
    assert!(systems[0].contains("  - Moon a"));
    let (bio, minerals) = systems[0]
        .lines()
        .filter(|line| line.starts_with("          Bio:"))
        .map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            assert_eq!((fields[0], fields[2]), ("Bio:", "Min:"));
            (
                fields[1].parse::<u16>().unwrap(),
                fields[3].parse::<u16>().unwrap(),
            )
        })
        .fold((0u16, 0u16), |(bio, minerals), (b, m)| {
            (bio.wrapping_add(b), minerals.wrapping_add(m))
        });
    assert!(
        minerals > 0,
        "the real default system must contain minerals"
    );
    let mut tally = Vec::new();
    tally_resources(&mut cx, &mut io, &mut generation, &mut tally).unwrap();
    assert_eq!(
        tally,
        format!("Name\t{minerals}\t{bio}\n")
            .repeat(NUM_SOLAR_SYSTEMS)
            .as_bytes()
    );
    for failed in [
        dump_universe(&mut cx, &mut io, &mut generation, &mut Broken),
        tally_resources(&mut cx, &mut io, &mut generation, &mut Broken),
    ] {
        assert_eq!(failed.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    }
    assert_eq!(cx.world.current_star, Some(StarId(7)));
    assert_eq!(cx.world.activity(), 0x1234);
    assert!(cx.kernel.diagnostics.interactive());
    assert!(cx.solar_frame.is_none());
}
