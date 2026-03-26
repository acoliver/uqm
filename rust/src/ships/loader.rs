// Two-Tier Ship Loader
// @plan PLAN-20260314-SHIPS.P05
// @requirement REQ-METADATA-LOAD, REQ-BATTLE-LOAD, REQ-DESCRIPTOR-FREE, REQ-LOAD-FAILURE

use super::c_bridge::{
    free_graphic, free_music, free_sound, free_string_table, load_graphic, load_music, load_sound,
    load_string_table, DrawableHandle, NULL_RESOURCE,
};
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
fn load_animation(res_ids: &[u32; NUM_VIEWS]) -> Result<[DrawableHandle; NUM_VIEWS], ShipsError> {
    if res_ids[0] == NULL_RESOURCE {
        return Err(ShipsError::LoadFailed(
            "load_animation: big_res (index 0) is NULL_RESOURCE".to_string(),
        ));
    }

    let big = load_graphic(res_ids[0])?;

    let med = if res_ids[1] == NULL_RESOURCE {
        big
    } else {
        load_graphic(res_ids[1])?
    };

    let sml = if res_ids[2] == NULL_RESOURCE {
        med
    } else {
        load_graphic(res_ids[2])?
    };

    Ok([big, med, sml])
}

/// Frees a 3-resolution animation array, avoiding double-free for shared handles.
///
/// Matches C `free_image()` behavior from `init.c`.
fn free_image(pixarray: &mut [DrawableHandle; NUM_VIEWS]) {
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
                free_graphic(pixarray[i]);
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
pub fn load_ship(species: SpeciesId, tier: LoadTier) -> Result<RaceDesc, ShipsError> {
    let mut desc = create_metadata_only_desc(species)?;

    // Track what we've loaded for cleanup on failure
    let mut loaded_icons = false;
    let mut loaded_melee_icon = false;
    let mut loaded_race_strings = false;
    let mut loaded_ship = false;
    let mut loaded_weapon = false;
    let mut loaded_special = false;
    let mut loaded_captain = false;
    let mut loaded_victory = false;
    let mut loaded_sounds = false;

    // Load metadata assets
    if desc.ship_info.icons_res != NULL_RESOURCE {
        match load_graphic(desc.ship_info.icons_res) {
            Ok(handle) => {
                desc.ship_info.icons = handle;
                loaded_icons = true;
            }
            Err(e) => {
                cleanup_partial_load(
                    &mut desc,
                    loaded_icons,
                    loaded_melee_icon,
                    loaded_race_strings,
                    loaded_ship,
                    loaded_weapon,
                    loaded_special,
                    loaded_captain,
                    loaded_victory,
                    loaded_sounds,
                );
                return Err(e);
            }
        }
    }

    if desc.ship_info.melee_icon_res != NULL_RESOURCE {
        match load_graphic(desc.ship_info.melee_icon_res) {
            Ok(handle) => {
                desc.ship_info.melee_icon = handle;
                loaded_melee_icon = true;
            }
            Err(e) => {
                cleanup_partial_load(
                    &mut desc,
                    loaded_icons,
                    loaded_melee_icon,
                    loaded_race_strings,
                    loaded_ship,
                    loaded_weapon,
                    loaded_special,
                    loaded_captain,
                    loaded_victory,
                    loaded_sounds,
                );
                return Err(e);
            }
        }
    }

    if desc.ship_info.race_strings_res != NULL_RESOURCE {
        match load_string_table(desc.ship_info.race_strings_res) {
            Ok(handle) => {
                desc.ship_info.race_strings = handle;
                loaded_race_strings = true;
            }
            Err(e) => {
                cleanup_partial_load(
                    &mut desc,
                    loaded_icons,
                    loaded_melee_icon,
                    loaded_race_strings,
                    loaded_ship,
                    loaded_weapon,
                    loaded_special,
                    loaded_captain,
                    loaded_victory,
                    loaded_sounds,
                );
                return Err(e);
            }
        }
    }

    // If MetadataOnly, we're done
    if tier == LoadTier::MetadataOnly {
        return Ok(desc);
    }

    // Load battle assets for BattleReady tier
    if desc.ship_data.ship_res[0] != NULL_RESOURCE {
        match load_animation(&desc.ship_data.ship_res) {
            Ok(frames) => {
                desc.ship_data.ship = frames;
                loaded_ship = true;
            }
            Err(e) => {
                cleanup_partial_load(
                    &mut desc,
                    loaded_icons,
                    loaded_melee_icon,
                    loaded_race_strings,
                    loaded_ship,
                    loaded_weapon,
                    loaded_special,
                    loaded_captain,
                    loaded_victory,
                    loaded_sounds,
                );
                return Err(e);
            }
        }
    }

    if desc.ship_data.weapon_res[0] != NULL_RESOURCE {
        match load_animation(&desc.ship_data.weapon_res) {
            Ok(frames) => {
                desc.ship_data.weapon = frames;
                loaded_weapon = true;
            }
            Err(e) => {
                cleanup_partial_load(
                    &mut desc,
                    loaded_icons,
                    loaded_melee_icon,
                    loaded_race_strings,
                    loaded_ship,
                    loaded_weapon,
                    loaded_special,
                    loaded_captain,
                    loaded_victory,
                    loaded_sounds,
                );
                return Err(e);
            }
        }
    }

    if desc.ship_data.special_res[0] != NULL_RESOURCE {
        match load_animation(&desc.ship_data.special_res) {
            Ok(frames) => {
                desc.ship_data.special = frames;
                loaded_special = true;
            }
            Err(e) => {
                cleanup_partial_load(
                    &mut desc,
                    loaded_icons,
                    loaded_melee_icon,
                    loaded_race_strings,
                    loaded_ship,
                    loaded_weapon,
                    loaded_special,
                    loaded_captain,
                    loaded_victory,
                    loaded_sounds,
                );
                return Err(e);
            }
        }
    }

    if desc.ship_data.captain.captain_res != NULL_RESOURCE {
        match load_graphic(desc.ship_data.captain.captain_res) {
            Ok(handle) => {
                desc.ship_data.captain.background = handle;
                loaded_captain = true;
            }
            Err(e) => {
                cleanup_partial_load(
                    &mut desc,
                    loaded_icons,
                    loaded_melee_icon,
                    loaded_race_strings,
                    loaded_ship,
                    loaded_weapon,
                    loaded_special,
                    loaded_captain,
                    loaded_victory,
                    loaded_sounds,
                );
                return Err(e);
            }
        }
    }

    if desc.ship_data.victory_ditty_res != NULL_RESOURCE {
        match load_music(desc.ship_data.victory_ditty_res) {
            Ok(handle) => {
                desc.ship_data.victory_ditty = handle;
                loaded_victory = true;
            }
            Err(e) => {
                cleanup_partial_load(
                    &mut desc,
                    loaded_icons,
                    loaded_melee_icon,
                    loaded_race_strings,
                    loaded_ship,
                    loaded_weapon,
                    loaded_special,
                    loaded_captain,
                    loaded_victory,
                    loaded_sounds,
                );
                return Err(e);
            }
        }
    }

    if desc.ship_data.ship_sounds_res != NULL_RESOURCE {
        match load_sound(desc.ship_data.ship_sounds_res) {
            Ok(handle) => {
                desc.ship_data.ship_sounds = handle;
                loaded_sounds = true;
            }
            Err(e) => {
                cleanup_partial_load(
                    &mut desc,
                    loaded_icons,
                    loaded_melee_icon,
                    loaded_race_strings,
                    loaded_ship,
                    loaded_weapon,
                    loaded_special,
                    loaded_captain,
                    loaded_victory,
                    loaded_sounds,
                );
                return Err(e);
            }
        }
    }

    // loaded_sounds is used by cleanup_partial_load in error paths above;
    // the final assignment is dead but required for structural consistency.
    let _ = loaded_sounds;

    Ok(desc)
}

/// Frees a ship descriptor's loaded assets.
///
/// # Parameters
/// - `desc`: The descriptor to free assets from.
/// - `free_icon_data`: If true, frees icons, melee_icon, race_strings.
/// - `free_battle_data`: If true, frees ship/weapon/special frames, captain, victory, sounds.
///
/// # Behavior
/// - Calls `desc.behavior.uninit()` first (teardown hook).
/// - Frees requested asset tiers and zeroes out handles.
/// - Safe to call multiple times (handles already zero are skipped).
pub fn free_ship(desc: &mut RaceDesc, free_icon_data: bool, free_battle_data: bool) {
    // Call teardown hook first
    desc.behavior.uninit();

    if free_battle_data {
        free_image(&mut desc.ship_data.ship);
        free_image(&mut desc.ship_data.weapon);
        free_image(&mut desc.ship_data.special);

        if desc.ship_data.captain.background != 0 {
            free_graphic(desc.ship_data.captain.background);
            desc.ship_data.captain.background = 0;
        }

        if desc.ship_data.victory_ditty != 0 {
            free_music(desc.ship_data.victory_ditty);
            desc.ship_data.victory_ditty = 0;
        }

        if desc.ship_data.ship_sounds != 0 {
            free_sound(desc.ship_data.ship_sounds);
            desc.ship_data.ship_sounds = 0;
        }
    }

    if free_icon_data {
        if desc.ship_info.icons != 0 {
            free_graphic(desc.ship_info.icons);
            desc.ship_info.icons = 0;
        }

        if desc.ship_info.melee_icon != 0 {
            free_graphic(desc.ship_info.melee_icon);
            desc.ship_info.melee_icon = 0;
        }

        if desc.ship_info.race_strings != 0 {
            free_string_table(desc.ship_info.race_strings);
            desc.ship_info.race_strings = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Internal Cleanup Helper
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn cleanup_partial_load(
    desc: &mut RaceDesc,
    loaded_icons: bool,
    loaded_melee_icon: bool,
    loaded_race_strings: bool,
    loaded_ship: bool,
    loaded_weapon: bool,
    loaded_special: bool,
    loaded_captain: bool,
    loaded_victory: bool,
    loaded_sounds: bool,
) {
    if loaded_sounds {
        free_sound(desc.ship_data.ship_sounds);
        desc.ship_data.ship_sounds = 0;
    }

    if loaded_victory {
        free_music(desc.ship_data.victory_ditty);
        desc.ship_data.victory_ditty = 0;
    }

    if loaded_captain {
        free_graphic(desc.ship_data.captain.background);
        desc.ship_data.captain.background = 0;
    }

    if loaded_special {
        free_image(&mut desc.ship_data.special);
    }

    if loaded_weapon {
        free_image(&mut desc.ship_data.weapon);
    }

    if loaded_ship {
        free_image(&mut desc.ship_data.ship);
    }

    if loaded_race_strings {
        free_string_table(desc.ship_info.race_strings);
        desc.ship_info.race_strings = 0;
    }

    if loaded_melee_icon {
        free_graphic(desc.ship_info.melee_icon);
        desc.ship_info.melee_icon = 0;
    }

    if loaded_icons {
        free_graphic(desc.ship_info.icons);
        desc.ship_info.icons = 0;
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ships::types::SpeciesId;

    // -- LoadTier enum tests ------------------------------------------------

    #[test]
    fn load_tier_has_two_variants() {
        unsafe {
            let metadata = LoadTier::MetadataOnly;
            let battle = LoadTier::BattleReady;
            assert_ne!(metadata, battle);
        }
    }

    #[test]
    fn load_tier_debug_format() {
        unsafe {
            let s = format!("{:?}", LoadTier::MetadataOnly);
            assert!(s.contains("MetadataOnly"));
        }
    }

    // -- MetadataOnly loading tests -----------------------------------------

    #[test]
    fn load_ship_metadata_only_all_28_species_succeed() {
        unsafe {
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
                let result = load_ship(*species, LoadTier::MetadataOnly);
                assert!(
                    result.is_ok(),
                    "load_ship MetadataOnly failed for {:?}",
                    species
                );
            }
        }
    }

    #[test]
    fn load_ship_metadata_only_does_not_load_battle_assets() {
        unsafe {
            let desc = load_ship(SpeciesId::Arilou, LoadTier::MetadataOnly).unwrap();
            assert_eq!(desc.ship_data.ship, [0; NUM_VIEWS]);
            assert_eq!(desc.ship_data.weapon, [0; NUM_VIEWS]);
            assert_eq!(desc.ship_data.special, [0; NUM_VIEWS]);
            assert_eq!(desc.ship_data.captain.background, 0);
            assert_eq!(desc.ship_data.victory_ditty, 0);
            assert_eq!(desc.ship_data.ship_sounds, 0);
        }
    }

    #[test]
    fn load_ship_metadata_only_no_id_fails() {
        unsafe {
            let result = load_ship(SpeciesId::NoId, LoadTier::MetadataOnly);
            assert!(result.is_err());
        }
    }

    // -- BattleReady loading tests ------------------------------------------

    #[test]
    fn load_ship_battle_ready_all_28_species_succeed() {
        unsafe {
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
                let result = load_ship(*species, LoadTier::BattleReady);
                assert!(
                    result.is_ok(),
                    "load_ship BattleReady failed for {:?}",
                    species
                );
            }
        }
    }

    #[test]
    fn load_ship_battle_ready_no_id_fails() {
        unsafe {
            let result = load_ship(SpeciesId::NoId, LoadTier::BattleReady);
            assert!(result.is_err());
        }
    }

    // -- free_ship tests ----------------------------------------------------

    #[test]
    fn free_ship_calls_uninit() {
        unsafe {
            use crate::ships::traits::ShipBehavior;
            use crate::ships::types::{Characteristics, FleetStuff, IntelStuff, RaceDescTemplate};
            use std::sync::atomic::{AtomicBool, Ordering};
            use std::sync::Arc;

            #[derive(Debug)]
            struct UninitTrackingBehavior {
                uninit_called: Arc<AtomicBool>,
            }

            impl ShipBehavior for UninitTrackingBehavior {
                fn descriptor_template(&self) -> RaceDescTemplate {
                    RaceDescTemplate {
                        ship_info: Default::default(),
                        fleet: FleetStuff::default(),
                        characteristics: Characteristics::default(),
                        ship_data: Default::default(),
                        intel: IntelStuff::default(),
                    }
                }

                fn uninit(&mut self) {
                    self.uninit_called.store(true, Ordering::SeqCst);
                }
            }

            let uninit_called = Arc::new(AtomicBool::new(false));
            let mut desc = RaceDesc {
                ship_info: Default::default(),
                fleet: FleetStuff::default(),
                characteristics: Characteristics::default(),
                ship_data: Default::default(),
                intel: IntelStuff::default(),
                behavior: Box::new(UninitTrackingBehavior {
                    uninit_called: Arc::clone(&uninit_called),
                }),
                data: None,
            };

            free_ship(&mut desc, false, false);
            assert!(uninit_called.load(Ordering::SeqCst));
        }
    }

    #[test]
    fn free_ship_metadata_only() {
        unsafe {
            let mut desc = load_ship(SpeciesId::Arilou, LoadTier::MetadataOnly).unwrap();
            free_ship(&mut desc, true, false);
            assert_eq!(desc.ship_info.icons, 0);
            assert_eq!(desc.ship_info.melee_icon, 0);
            assert_eq!(desc.ship_info.race_strings, 0);
        }
    }

    #[test]
    fn free_ship_battle_only() {
        unsafe {
            let mut desc = load_ship(SpeciesId::Arilou, LoadTier::BattleReady).unwrap();
            free_ship(&mut desc, false, true);
            assert_eq!(desc.ship_data.ship, [0; NUM_VIEWS]);
            assert_eq!(desc.ship_data.weapon, [0; NUM_VIEWS]);
            assert_eq!(desc.ship_data.special, [0; NUM_VIEWS]);
            assert_eq!(desc.ship_data.captain.background, 0);
            assert_eq!(desc.ship_data.victory_ditty, 0);
            assert_eq!(desc.ship_data.ship_sounds, 0);
        }
    }

    #[test]
    fn free_ship_both_tiers() {
        unsafe {
            let mut desc = load_ship(SpeciesId::Chmmr, LoadTier::BattleReady).unwrap();
            free_ship(&mut desc, true, true);
            assert_eq!(desc.ship_info.icons, 0);
            assert_eq!(desc.ship_data.ship, [0; NUM_VIEWS]);
        }
    }

    // -- free_image tests ---------------------------------------------------

    #[test]
    fn free_image_avoids_double_free_for_shared_handles() {
        unsafe {
            let mut pixarray = [100, 100, 200];
            free_image(&mut pixarray);
            assert_eq!(pixarray, [0, 0, 0]);
        }
    }

    #[test]
    fn free_image_handles_all_unique() {
        unsafe {
            let mut pixarray = [100, 200, 300];
            free_image(&mut pixarray);
            assert_eq!(pixarray, [0, 0, 0]);
        }
    }

    #[test]
    fn free_image_handles_all_same() {
        unsafe {
            let mut pixarray = [100, 100, 100];
            free_image(&mut pixarray);
            assert_eq!(pixarray, [0, 0, 0]);
        }
    }

    #[test]
    fn free_image_handles_zeroes() {
        unsafe {
            let mut pixarray = [0, 0, 0];
            free_image(&mut pixarray);
            assert_eq!(pixarray, [0, 0, 0]);
        }
    }

    // -- load_animation tests -----------------------------------------------

    #[test]
    fn load_animation_null_big_res_fails() {
        unsafe {
            crate::ships::c_bridge::mock_reset();
            let res_ids = [NULL_RESOURCE, 0, 0];
            let result = load_animation(&res_ids);
            assert!(result.is_err());
        }
    }

    #[test]
    fn load_animation_valid_big_res_succeeds() {
        unsafe {
            crate::ships::c_bridge::mock_reset();
            let res_ids = [1, NULL_RESOURCE, NULL_RESOURCE];
            let result = load_animation(&res_ids);
            assert!(result.is_ok());
            let handles = result.unwrap();
            assert_ne!(handles[0], 0);
            assert_eq!(handles[0], handles[1]);
            assert_eq!(handles[1], handles[2]);
        }
    }

    #[test]
    fn load_animation_all_unique_res_ids() {
        unsafe {
            crate::ships::c_bridge::mock_reset();
            let res_ids = [1, 2, 3];
            let result = load_animation(&res_ids);
            assert!(result.is_ok());
            let handles = result.unwrap();
            assert_ne!(handles[0], 0);
            assert_ne!(handles[1], 0);
            assert_ne!(handles[2], 0);
        }
    }

    #[test]
    fn load_animation_null_med_reuses_big() {
        unsafe {
            crate::ships::c_bridge::mock_reset();
            let res_ids = [1, NULL_RESOURCE, 3];
            let result = load_animation(&res_ids);
            assert!(result.is_ok());
            let handles = result.unwrap();
            assert_eq!(handles[0], handles[1]);
            assert_ne!(handles[1], handles[2]);
        }
    }

    #[test]
    fn load_animation_null_sml_reuses_med() {
        unsafe {
            crate::ships::c_bridge::mock_reset();
            let res_ids = [1, 2, NULL_RESOURCE];
            let result = load_animation(&res_ids);
            assert!(result.is_ok());
            let handles = result.unwrap();
            assert_ne!(handles[0], handles[1]);
            assert_eq!(handles[1], handles[2]);
        }
    }

    // -- Partial failure cleanup tests (using failure injection) ---------------

    #[test]
    fn partial_load_failure_cleans_up_icons_when_melee_icon_fails() {
        unsafe {
            use crate::ships::c_bridge::{
                mock_allocated_count, mock_reset, mock_set_fail_on_res_id,
            };
            use crate::ships::types::{
                Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags,
                ShipInfo,
            };

            mock_reset();

            // Build a descriptor with non-zero resource IDs to trigger real loads
            let mut desc = create_metadata_only_desc(SpeciesId::Arilou).unwrap();
            desc.ship_info.icons_res = 100;
            desc.ship_info.melee_icon_res = 200;
            desc.ship_info.race_strings_res = 300;

            // Icons (100) will load, melee_icon (200) will fail
            mock_set_fail_on_res_id(200);

            // We can't use load_ship directly since it creates its own desc.
            // Instead test cleanup_partial_load via load_ship with modified template.
            // First load icons manually to simulate the partial state.
            let icon_handle = crate::ships::c_bridge::load_graphic(100).unwrap();
            assert!(crate::ships::c_bridge::mock_is_allocated(icon_handle));

            // Set it on the desc and simulate cleanup
            desc.ship_info.icons = icon_handle;
            cleanup_partial_load(
                &mut desc, true, false, false, false, false, false, false, false, false,
            );

            // Icons should be freed
            assert!(!crate::ships::c_bridge::mock_is_allocated(icon_handle));
            assert_eq!(desc.ship_info.icons, 0);
            assert_eq!(mock_allocated_count(), 0);
            mock_reset();
        }
    }

    #[test]
    fn partial_load_failure_cleans_up_metadata_when_battle_fails() {
        unsafe {
            use crate::ships::c_bridge::{mock_allocated_count, mock_reset};

            mock_reset();

            // Manually simulate: icons + melee + strings loaded, then battle fails
            let h_icons = crate::ships::c_bridge::load_graphic(100).unwrap();
            let h_melee = crate::ships::c_bridge::load_graphic(101).unwrap();
            let h_strings = crate::ships::c_bridge::load_string_table(102).unwrap();
            assert_eq!(mock_allocated_count(), 3);

            let mut desc = create_metadata_only_desc(SpeciesId::Chmmr).unwrap();
            desc.ship_info.icons = h_icons;
            desc.ship_info.melee_icon = h_melee;
            desc.ship_info.race_strings = h_strings;

            // Simulate battle failure: cleanup should free all metadata
            cleanup_partial_load(
                &mut desc, true, true, true, false, false, false, false, false, false,
            );
            assert_eq!(mock_allocated_count(), 0);
            assert_eq!(desc.ship_info.icons, 0);
            assert_eq!(desc.ship_info.melee_icon, 0);
            assert_eq!(desc.ship_info.race_strings, 0);
            mock_reset();
        }
    }

    #[test]
    fn partial_load_failure_cleans_up_battle_assets_and_metadata() {
        unsafe {
            use crate::ships::c_bridge::{mock_allocated_count, mock_reset};

            mock_reset();

            let h_icons = crate::ships::c_bridge::load_graphic(100).unwrap();
            let h_ship_big = crate::ships::c_bridge::load_graphic(201).unwrap();
            let h_ship_med = crate::ships::c_bridge::load_graphic(202).unwrap();
            let h_captain = crate::ships::c_bridge::load_graphic(300).unwrap();
            assert_eq!(mock_allocated_count(), 4);

            let mut desc = create_metadata_only_desc(SpeciesId::Earthling).unwrap();
            desc.ship_info.icons = h_icons;
            desc.ship_data.ship = [h_ship_big, h_ship_med, h_ship_med]; // med shared with sml
            desc.ship_data.captain.background = h_captain;

            cleanup_partial_load(
                &mut desc, true, false, false, true, false, false, true, false, false,
            );

            // All should be freed (ship handles with double-free avoidance)
            assert_eq!(mock_allocated_count(), 0);
            assert_eq!(desc.ship_info.icons, 0);
            assert_eq!(desc.ship_data.ship, [0, 0, 0]);
            assert_eq!(desc.ship_data.captain.background, 0);
            mock_reset();
        }
    }

    // -- Double-free protection with tracking ---------------------------------

    #[test]
    fn free_image_shared_handles_freed_exactly_once() {
        unsafe {
            use crate::ships::c_bridge::{mock_free_count, mock_reset};

            mock_reset();
            let h1 = crate::ships::c_bridge::load_graphic(1).unwrap();
            // Simulate med and sml reusing big handle
            let mut pixarray = [h1, h1, h1];
            free_image(&mut pixarray);

            // Handle freed exactly once despite appearing 3 times
            assert_eq!(mock_free_count(h1), 1);
            assert_eq!(pixarray, [0, 0, 0]);
            mock_reset();
        }
    }

    #[test]
    fn free_image_two_shared_one_unique_freed_correctly() {
        unsafe {
            use crate::ships::c_bridge::{mock_free_count, mock_reset};

            mock_reset();
            let h1 = crate::ships::c_bridge::load_graphic(1).unwrap();
            let h2 = crate::ships::c_bridge::load_graphic(2).unwrap();
            // big=h1, med=h1, sml=h2
            let mut pixarray = [h1, h1, h2];
            free_image(&mut pixarray);

            assert_eq!(mock_free_count(h1), 1);
            assert_eq!(mock_free_count(h2), 1);
            assert_eq!(pixarray, [0, 0, 0]);
            mock_reset();
        }
    }

    // -- free_ship with allocation tracking -----------------------------------

    #[test]
    fn free_ship_no_leaks_after_metadata_free() {
        unsafe {
            use crate::ships::c_bridge::{mock_allocated_count, mock_reset};

            mock_reset();
            let h_icons = crate::ships::c_bridge::load_graphic(10).unwrap();
            let h_melee = crate::ships::c_bridge::load_graphic(11).unwrap();
            let h_strings = crate::ships::c_bridge::load_string_table(12).unwrap();

            let mut desc = create_metadata_only_desc(SpeciesId::Spathi).unwrap();
            desc.ship_info.icons = h_icons;
            desc.ship_info.melee_icon = h_melee;
            desc.ship_info.race_strings = h_strings;
            assert_eq!(mock_allocated_count(), 3);

            free_ship(&mut desc, true, false);
            assert_eq!(mock_allocated_count(), 0);
            mock_reset();
        }
    }
}
