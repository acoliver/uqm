// Two-Tier Ship Loader
// @plan PLAN-20260314-SHIPS.P05
// @requirement REQ-METADATA-LOAD, REQ-BATTLE-LOAD, REQ-DESCRIPTOR-FREE, REQ-LOAD-FAILURE

use super::c_bridge::{DrawableHandle, ResourceBridge, NULL_RESOURCE};
use super::registry::create_metadata_only_desc;
use super::types::{RaceDesc, ShipsError, SpeciesId, NUM_VIEWS};

// ---------------------------------------------------------------------------
// LoadTier enum
// ---------------------------------------------------------------------------

/// Specifies which tier of ship assets to load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadTier {
    /// Load only metadata: icons, melee icons, race strings.
    /// Suitable for catalog, fleet management, ship selection UI.
    MetadataOnly,

    /// Load all metadata plus battle assets: ship/weapon/special frames,
    /// captain graphics, victory music, ship sounds.
    /// Required for combat/melee.
    BattleReady,
}

// ---------------------------------------------------------------------------
// Resource Loading Helpers
// ---------------------------------------------------------------------------

/// Loads a 3-resolution animation following C `load_animation()` semantics.
///
/// - If `res_ids[0]` is NULL_RESOURCE, returns error (big res is mandatory).
/// - If `res_ids[1]` is NULL_RESOURCE, reuses `res_ids[0]` handle.
/// - If `res_ids[2]` is NULL_RESOURCE, reuses `res_ids[1]` handle.
///
/// This matches the C behavior from `init.c::load_animation()`.
///
/// # Errors
/// Returns `ShipsError::LoadFailed` if loading fails.
fn load_animation(
    res: &mut impl ResourceBridge,
    res_ids: &[u32; NUM_VIEWS],
) -> Result<[DrawableHandle; NUM_VIEWS], ShipsError> {
    if res_ids[0] == NULL_RESOURCE {
        return Err(ShipsError::LoadFailed(
            "load_animation: big_res (index 0) is NULL_RESOURCE".to_string(),
        ));
    }

    let big = res.load_graphic(res_ids[0])?;

    let med = if res_ids[1] == NULL_RESOURCE {
        big
    } else {
        res.load_graphic(res_ids[1])?
    };

    let sml = if res_ids[2] == NULL_RESOURCE {
        med
    } else {
        res.load_graphic(res_ids[2])?
    };

    Ok([big, med, sml])
}

/// Frees a 3-resolution animation array, avoiding double-free for shared handles.
///
/// Matches C `free_image()` behavior from `init.c`.
fn free_image(res: &mut impl ResourceBridge, pixarray: &mut [DrawableHandle; NUM_VIEWS]) {
    let mut already_freed: [DrawableHandle; NUM_VIEWS] = [0; NUM_VIEWS];

    for i in 0..NUM_VIEWS {
        if pixarray[i] != 0 {
            let mut ok = true;
            for freed in already_freed.iter().take(i) {
                if *freed == pixarray[i] {
                    ok = false;
                    break;
                }
            }
            if ok {
                res.free_graphic(pixarray[i]);
            }
            already_freed[i] = pixarray[i];
            pixarray[i] = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Ship Loading
// ---------------------------------------------------------------------------

/// Loads a ship descriptor at the specified tier.
///
/// # Tier Behavior
///
/// ## `LoadTier::MetadataOnly`
/// Loads only:
/// - Icons (if `icons_res` != 0)
/// - Melee icon (if `melee_icon_res` != 0)
/// - Race strings (if `race_strings_res` != 0)
///
/// Does NOT load battle assets. Safe for catalog/fleet/UI work.
///
/// ## `LoadTier::BattleReady`
/// Loads all metadata assets PLUS:
/// - Ship frames (3 resolutions)
/// - Weapon frames (3 resolutions, if `weapon_res[0]` != 0)
/// - Special frames (3 resolutions, if `special_res[0]` != 0)
/// - Captain background (if `captain_res` != 0)
/// - Victory music (if `victory_ditty_res` != 0)
/// - Ship sounds (if `ship_sounds_res` != 0)
///
/// # Cleanup on Failure
/// If any resource load fails, all previously loaded resources are freed
/// before returning `Err`.
///
/// # Errors
/// - `ShipsError::UnknownSpecies` if `species` is invalid.
/// - `ShipsError::LoadFailed` if any resource load fails.
pub fn load_ship(
    res: &mut impl ResourceBridge,
    species: SpeciesId,
    tier: LoadTier,
) -> Result<RaceDesc, ShipsError> {
    let mut desc = create_metadata_only_desc(species)?;
    // A fresh descriptor holds no handles, so every non-zero handle after a
    // failure was loaded here and free_ship releases exactly those.
    if let Err(error) = load_assets(res, &mut desc, tier) {
        free_ship(res, &mut desc, true, true);
        return Err(error);
    }
    Ok(desc)
}

fn load_assets(
    res: &mut impl ResourceBridge,
    desc: &mut RaceDesc,
    tier: LoadTier,
) -> Result<(), ShipsError> {
    let info = &mut desc.ship_info;
    if info.icons_res != NULL_RESOURCE {
        info.icons = res.load_graphic(info.icons_res)?;
    }
    if info.melee_icon_res != NULL_RESOURCE {
        info.melee_icon = res.load_graphic(info.melee_icon_res)?;
    }
    if info.race_strings_res != NULL_RESOURCE {
        info.race_strings = res.load_string_table(info.race_strings_res)?;
    }
    if tier == LoadTier::MetadataOnly {
        return Ok(());
    }

    let data = &mut desc.ship_data;
    if data.ship_res[0] != NULL_RESOURCE {
        data.ship = load_animation(res, &data.ship_res)?;
    }
    if data.weapon_res[0] != NULL_RESOURCE {
        data.weapon = load_animation(res, &data.weapon_res)?;
    }
    if data.special_res[0] != NULL_RESOURCE {
        data.special = load_animation(res, &data.special_res)?;
    }
    if data.captain.captain_res != NULL_RESOURCE {
        data.captain.background = res.load_graphic(data.captain.captain_res)?;
    }
    if data.victory_ditty_res != NULL_RESOURCE {
        data.victory_ditty = res.load_music(data.victory_ditty_res)?;
    }
    if data.ship_sounds_res != NULL_RESOURCE {
        data.ship_sounds = res.load_sound(data.ship_sounds_res)?;
    }
    Ok(())
}

/// Frees a ship descriptor's loaded assets.
///
/// # Parameters
/// - `desc`: The descriptor to free assets from.
/// - `free_icon_data`: If true, frees icons, melee_icon, race_strings.
/// - `free_battle_data`: If true, frees ship/weapon/special frames, captain, victory, sounds.
///
/// # Behavior
/// - Frees requested asset tiers and zeroes out handles.
/// - Safe to call multiple times (handles already zero are skipped).
pub fn free_ship(
    res: &mut impl ResourceBridge,
    desc: &mut RaceDesc,
    free_icon_data: bool,
    free_battle_data: bool,
) {
    // Call teardown hook first

    if free_battle_data {
        free_image(res, &mut desc.ship_data.ship);
        free_image(res, &mut desc.ship_data.weapon);
        free_image(res, &mut desc.ship_data.special);

        if desc.ship_data.captain.background != 0 {
            res.free_graphic(desc.ship_data.captain.background);
            desc.ship_data.captain.background = 0;
        }

        if desc.ship_data.victory_ditty != 0 {
            res.free_music(desc.ship_data.victory_ditty);
            desc.ship_data.victory_ditty = 0;
        }

        if desc.ship_data.ship_sounds != 0 {
            res.free_sound(desc.ship_data.ship_sounds);
            desc.ship_data.ship_sounds = 0;
        }
    }

    if free_icon_data {
        if desc.ship_info.icons != 0 {
            res.free_graphic(desc.ship_info.icons);
            desc.ship_info.icons = 0;
        }

        if desc.ship_info.melee_icon != 0 {
            res.free_graphic(desc.ship_info.melee_icon);
            desc.ship_info.melee_icon = 0;
        }

        if desc.ship_info.race_strings != 0 {
            res.free_string_table(desc.ship_info.race_strings);
            desc.ship_info.race_strings = 0;
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ships::c_bridge::mock::MockResources;
    use crate::ships::types::SpeciesId;

    // -- LoadTier enum tests ------------------------------------------------

    #[test]
    fn load_tier_has_two_variants() {
        let metadata = LoadTier::MetadataOnly;
        let battle = LoadTier::BattleReady;
        assert_ne!(metadata, battle);
    }

    #[test]
    fn load_tier_debug_format() {
        let s = format!("{:?}", LoadTier::MetadataOnly);
        assert!(s.contains("MetadataOnly"));
    }

    // -- MetadataOnly loading tests -----------------------------------------

    #[test]
    fn load_ship_metadata_only_all_28_species_succeed() {
        let mut res = MockResources::default();
        let all_species = [
            SpeciesId::Arilou,
            SpeciesId::Chmmr,
            SpeciesId::Earthling,
            SpeciesId::Orz,
            SpeciesId::Pkunk,
            SpeciesId::Shofixti,
            SpeciesId::Spathi,
            SpeciesId::Supox,
            SpeciesId::Thraddash,
            SpeciesId::Utwig,
            SpeciesId::Vux,
            SpeciesId::Yehat,
            SpeciesId::Melnorme,
            SpeciesId::Druuge,
            SpeciesId::Ilwrath,
            SpeciesId::Mycon,
            SpeciesId::Slylandro,
            SpeciesId::Umgah,
            SpeciesId::UrQuan,
            SpeciesId::Zoqfotpik,
            SpeciesId::Syreen,
            SpeciesId::KohrAh,
            SpeciesId::Androsynth,
            SpeciesId::Chenjesu,
            SpeciesId::Mmrnmhrm,
            SpeciesId::SisShip,
            SpeciesId::SaMatra,
            SpeciesId::UrQuanProbe,
        ];
        for species in &all_species {
            let result = load_ship(&mut res, *species, LoadTier::MetadataOnly);
            assert!(
                result.is_ok(),
                "load_ship MetadataOnly failed for {:?}",
                species
            );
        }
    }

    #[test]
    fn load_ship_metadata_only_does_not_load_battle_assets() {
        let mut res = MockResources::default();
        let desc = load_ship(&mut res, SpeciesId::Arilou, LoadTier::MetadataOnly).unwrap();
        assert_eq!(desc.ship_data.ship, [0; NUM_VIEWS]);
        assert_eq!(desc.ship_data.weapon, [0; NUM_VIEWS]);
        assert_eq!(desc.ship_data.special, [0; NUM_VIEWS]);
        assert_eq!(desc.ship_data.captain.background, 0);
        assert_eq!(desc.ship_data.victory_ditty, 0);
        assert_eq!(desc.ship_data.ship_sounds, 0);
    }

    #[test]
    fn load_ship_metadata_only_no_id_fails() {
        let mut res = MockResources::default();
        let result = load_ship(&mut res, SpeciesId::NoId, LoadTier::MetadataOnly);
        assert!(result.is_err());
    }

    // -- BattleReady loading tests ------------------------------------------

    #[test]
    fn load_ship_battle_ready_all_28_species_succeed() {
        let mut res = MockResources::default();
        let all_species = [
            SpeciesId::Arilou,
            SpeciesId::Chmmr,
            SpeciesId::Earthling,
            SpeciesId::Orz,
            SpeciesId::Pkunk,
            SpeciesId::Shofixti,
            SpeciesId::Spathi,
            SpeciesId::Supox,
            SpeciesId::Thraddash,
            SpeciesId::Utwig,
            SpeciesId::Vux,
            SpeciesId::Yehat,
            SpeciesId::Melnorme,
            SpeciesId::Druuge,
            SpeciesId::Ilwrath,
            SpeciesId::Mycon,
            SpeciesId::Slylandro,
            SpeciesId::Umgah,
            SpeciesId::UrQuan,
            SpeciesId::Zoqfotpik,
            SpeciesId::Syreen,
            SpeciesId::KohrAh,
            SpeciesId::Androsynth,
            SpeciesId::Chenjesu,
            SpeciesId::Mmrnmhrm,
            SpeciesId::SisShip,
            SpeciesId::SaMatra,
            SpeciesId::UrQuanProbe,
        ];
        for species in &all_species {
            let result = load_ship(&mut res, *species, LoadTier::BattleReady);
            assert!(
                result.is_ok(),
                "load_ship BattleReady failed for {:?}",
                species
            );
        }
    }

    #[test]
    fn load_ship_battle_ready_no_id_fails() {
        let mut res = MockResources::default();
        let result = load_ship(&mut res, SpeciesId::NoId, LoadTier::BattleReady);
        assert!(result.is_err());
    }

    // -- free_ship tests ----------------------------------------------------

    #[test]
    fn free_ship_metadata_only() {
        let mut res = MockResources::default();
        let mut desc = load_ship(&mut res, SpeciesId::Arilou, LoadTier::MetadataOnly).unwrap();
        free_ship(&mut res, &mut desc, true, false);
        assert_eq!(desc.ship_info.icons, 0);
        assert_eq!(desc.ship_info.melee_icon, 0);
        assert_eq!(desc.ship_info.race_strings, 0);
    }

    #[test]
    fn free_ship_battle_only() {
        let mut res = MockResources::default();
        let mut desc = load_ship(&mut res, SpeciesId::Arilou, LoadTier::BattleReady).unwrap();
        free_ship(&mut res, &mut desc, false, true);
        assert_eq!(desc.ship_data.ship, [0; NUM_VIEWS]);
        assert_eq!(desc.ship_data.weapon, [0; NUM_VIEWS]);
        assert_eq!(desc.ship_data.special, [0; NUM_VIEWS]);
        assert_eq!(desc.ship_data.captain.background, 0);
        assert_eq!(desc.ship_data.victory_ditty, 0);
        assert_eq!(desc.ship_data.ship_sounds, 0);
    }

    #[test]
    fn free_ship_both_tiers() {
        let mut res = MockResources::default();
        let mut desc = load_ship(&mut res, SpeciesId::Chmmr, LoadTier::BattleReady).unwrap();
        free_ship(&mut res, &mut desc, true, true);
        assert_eq!(desc.ship_info.icons, 0);
        assert_eq!(desc.ship_data.ship, [0; NUM_VIEWS]);
    }

    // -- free_image tests ---------------------------------------------------

    #[test]
    fn free_image_avoids_double_free_for_shared_handles() {
        let mut res = MockResources::default();
        let mut pixarray = [100, 100, 200];
        free_image(&mut res, &mut pixarray);
        assert_eq!(pixarray, [0, 0, 0]);
    }

    #[test]
    fn free_image_handles_all_unique() {
        let mut res = MockResources::default();
        let mut pixarray = [100, 200, 300];
        free_image(&mut res, &mut pixarray);
        assert_eq!(pixarray, [0, 0, 0]);
    }

    #[test]
    fn free_image_handles_all_same() {
        let mut res = MockResources::default();
        let mut pixarray = [100, 100, 100];
        free_image(&mut res, &mut pixarray);
        assert_eq!(pixarray, [0, 0, 0]);
    }

    #[test]
    fn free_image_handles_zeroes() {
        let mut res = MockResources::default();
        let mut pixarray = [0, 0, 0];
        free_image(&mut res, &mut pixarray);
        assert_eq!(pixarray, [0, 0, 0]);
    }

    // -- load_animation tests -----------------------------------------------

    #[test]
    fn load_animation_null_big_res_fails() {
        let mut res = MockResources::default();
        let res_ids = [NULL_RESOURCE, 0, 0];
        let result = load_animation(&mut res, &res_ids);
        assert!(result.is_err());
    }

    #[test]
    fn load_animation_valid_big_res_succeeds() {
        let mut res = MockResources::default();
        let res_ids = [1, NULL_RESOURCE, NULL_RESOURCE];
        let result = load_animation(&mut res, &res_ids);
        assert!(result.is_ok());
        let handles = result.unwrap();
        assert_ne!(handles[0], 0);
        assert_eq!(handles[0], handles[1]);
        assert_eq!(handles[1], handles[2]);
    }

    #[test]
    fn load_animation_all_unique_res_ids() {
        let mut res = MockResources::default();
        let res_ids = [1, 2, 3];
        let result = load_animation(&mut res, &res_ids);
        assert!(result.is_ok());
        let handles = result.unwrap();
        assert_ne!(handles[0], 0);
        assert_ne!(handles[1], 0);
        assert_ne!(handles[2], 0);
    }

    #[test]
    fn load_animation_null_med_reuses_big() {
        let mut res = MockResources::default();
        let res_ids = [1, NULL_RESOURCE, 3];
        let result = load_animation(&mut res, &res_ids);
        assert!(result.is_ok());
        let handles = result.unwrap();
        assert_eq!(handles[0], handles[1]);
        assert_ne!(handles[1], handles[2]);
    }

    #[test]
    fn load_animation_null_sml_reuses_med() {
        let mut res = MockResources::default();
        let res_ids = [1, 2, NULL_RESOURCE];
        let result = load_animation(&mut res, &res_ids);
        assert!(result.is_ok());
        let handles = result.unwrap();
        assert_ne!(handles[0], handles[1]);
        assert_eq!(handles[1], handles[2]);
    }

    // -- Partial failure cleanup tests (using failure injection) ---------------

    #[test]
    fn partial_load_failure_cleans_up_icons_when_melee_icon_fails() {
        let mut res = MockResources::default();
        // Build a descriptor with non-zero resource IDs to trigger real loads
        let mut desc = create_metadata_only_desc(SpeciesId::Arilou).unwrap();
        desc.ship_info.icons_res = 100;
        desc.ship_info.melee_icon_res = 200;
        desc.ship_info.race_strings_res = 300;
        // Icons (100) will load, melee_icon (200) will fail
        res.fail_on(200);
        // Registry descriptors carry no resource ids, so build the partial
        // state load_ship leaves behind when the melee icon fails.
        let icon_handle = res.load_graphic(100).unwrap();
        assert!(res.is_allocated(icon_handle));
        // Set it on the desc and simulate cleanup
        desc.ship_info.icons = icon_handle;
        free_ship(&mut res, &mut desc, true, true);
        // Icons should be freed
        assert!(!res.is_allocated(icon_handle));
        assert_eq!(desc.ship_info.icons, 0);
        assert_eq!(res.allocated_count(), 0);
    }

    #[test]
    fn partial_load_failure_cleans_up_metadata_when_battle_fails() {
        let mut res = MockResources::default();
        // Manually simulate: icons + melee + strings loaded, then battle fails
        let h_icons = res.load_graphic(100).unwrap();
        let h_melee = res.load_graphic(101).unwrap();
        let h_strings = res.load_string_table(102).unwrap();
        assert_eq!(res.allocated_count(), 3);
        let mut desc = create_metadata_only_desc(SpeciesId::Chmmr).unwrap();
        desc.ship_info.icons = h_icons;
        desc.ship_info.melee_icon = h_melee;
        desc.ship_info.race_strings = h_strings;
        // Simulate battle failure: cleanup should free all metadata
        free_ship(&mut res, &mut desc, true, true);
        assert_eq!(res.allocated_count(), 0);
        assert_eq!(desc.ship_info.icons, 0);
        assert_eq!(desc.ship_info.melee_icon, 0);
        assert_eq!(desc.ship_info.race_strings, 0);
    }

    #[test]
    fn partial_load_failure_cleans_up_battle_assets_and_metadata() {
        let mut res = MockResources::default();
        let h_icons = res.load_graphic(100).unwrap();
        let h_ship_big = res.load_graphic(201).unwrap();
        let h_ship_med = res.load_graphic(202).unwrap();
        let h_captain = res.load_graphic(300).unwrap();
        assert_eq!(res.allocated_count(), 4);
        let mut desc = create_metadata_only_desc(SpeciesId::Earthling).unwrap();
        desc.ship_info.icons = h_icons;
        desc.ship_data.ship = [h_ship_big, h_ship_med, h_ship_med]; // med shared with sml
        desc.ship_data.captain.background = h_captain;
        free_ship(&mut res, &mut desc, true, true);
        // All should be freed (ship handles with double-free avoidance)
        assert_eq!(res.allocated_count(), 0);
        assert_eq!(desc.ship_info.icons, 0);
        assert_eq!(desc.ship_data.ship, [0, 0, 0]);
        assert_eq!(desc.ship_data.captain.background, 0);
    }

    // -- Double-free protection with tracking ---------------------------------

    #[test]
    fn free_image_shared_handles_freed_exactly_once() {
        let mut res = MockResources::default();
        let h1 = res.load_graphic(1).unwrap();
        // Simulate med and sml reusing big handle
        let mut pixarray = [h1, h1, h1];
        free_image(&mut res, &mut pixarray);
        // Handle freed exactly once despite appearing 3 times
        assert_eq!(res.free_count(h1), 1);
        assert_eq!(pixarray, [0, 0, 0]);
    }

    #[test]
    fn free_image_two_shared_one_unique_freed_correctly() {
        let mut res = MockResources::default();
        let h1 = res.load_graphic(1).unwrap();
        let h2 = res.load_graphic(2).unwrap();
        // big=h1, med=h1, sml=h2
        let mut pixarray = [h1, h1, h2];
        free_image(&mut res, &mut pixarray);
        assert_eq!(res.free_count(h1), 1);
        assert_eq!(res.free_count(h2), 1);
        assert_eq!(pixarray, [0, 0, 0]);
    }

    // -- free_ship with allocation tracking -----------------------------------

    #[test]
    fn free_ship_no_leaks_after_metadata_free() {
        let mut res = MockResources::default();
        let h_icons = res.load_graphic(10).unwrap();
        let h_melee = res.load_graphic(11).unwrap();
        let h_strings = res.load_string_table(12).unwrap();
        let mut desc = create_metadata_only_desc(SpeciesId::Spathi).unwrap();
        desc.ship_info.icons = h_icons;
        desc.ship_info.melee_icon = h_melee;
        desc.ship_info.race_strings = h_strings;
        assert_eq!(res.allocated_count(), 3);
        free_ship(&mut res, &mut desc, true, false);
        assert_eq!(res.allocated_count(), 0);
    }
}
