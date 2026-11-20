//! Resource ownership follows a descriptor through attachment and detachment.
use crate::battle::{frames::FrameArray, record::*, RaceDesc};
use crate::graphics::drawable::{FrameId, LoadedDrawable};
use crate::strings::owned_table::LoadedTextTable;

#[derive(Debug)]
pub struct ShipAssets {
    navigation: ShipNavigationAssets,
    captain: FrameId,
    victory: Option<crate::sound::resource_types::LoadedMusic>,
    sounds: Option<crate::sound::owned_bank::LoadedSoundBank>,
    animations: [FrameArray; 3],
}

struct ShipNavigationAssets {
    // Release authority follows melee icon, icons, then strings, including on campaign teardown.
    melee_icon: Option<LoadedDrawable>,
    icons: Option<LoadedDrawable>,
    strings: Option<LoadedTextTable>,
}

impl std::fmt::Debug for ShipNavigationAssets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShipNavigationAssets")
            .field(
                "melee_icon",
                &self.melee_icon.as_ref().map(LoadedDrawable::first_frame),
            )
            .field(
                "icons",
                &self.icons.as_ref().map(LoadedDrawable::first_frame),
            )
            .field("strings", &self.strings)
            .finish()
    }
}

/// The probe's loaded resources outlive its descriptor and belong to one campaign.
#[derive(Debug)]
pub struct CampaignProbeAssets {
    _navigation: ShipNavigationAssets,
}

fn icon_frame(owner: &Option<LoadedDrawable>) -> FrameId {
    owner
        .as_ref()
        .map_or(FrameId::NULL, LoadedDrawable::first_frame)
}

impl RaceDesc {
    pub fn label(&self, index: u16) -> (&std::ffi::CStr, u16) {
        self.raw.label(index)
    }

    pub fn picker_resources(&self) -> crate::battle::link_queue::ShipPickerResources {
        let assets = self
            .raw
            .assets
            .as_ref()
            .unwrap_or_else(|| panic!("picker resources require loaded assets"));
        crate::battle::link_queue::ShipPickerResources::new(
            assets
                .navigation
                .strings
                .as_ref()
                .and_then(|table| table.entry(0)),
            crate::battle::frames::FrameSelection::retain(icon_frame(&assets.navigation.icons)),
        )
    }
}

impl RaceDescriptor {
    pub fn label(&self, index: u16) -> (&std::ffi::CStr, u16) {
        self.assets
            .as_ref()
            .unwrap_or_else(|| panic!("ship labels require loaded assets"))
            .navigation
            .strings
            .as_ref()
            .map_or((c"", 0), |table| table.label(index))
    }
    pub fn sound_bank(&self) -> Option<&crate::sound::owned_bank::LoadedSoundBank> {
        self.assets
            .as_ref()
            .and_then(|assets| assets.sounds.as_ref())
    }

    pub fn victory_music(&self) -> Option<&crate::sound::resource_types::LoadedMusic> {
        self.assets
            .as_ref()
            .and_then(|assets| assets.victory.as_ref())
    }
    pub fn take_captain_background(&mut self) -> FrameId {
        let assets = self
            .assets
            .as_mut()
            .unwrap_or_else(|| panic!("captain release requires loaded ship assets"));
        self.ship_data.captain_control.background = FrameId::NULL;
        std::mem::replace(&mut assets.captain, FrameId::NULL)
    }
}

fn load_animation_set(keys: [Option<&'static std::ffi::CStr>; 3]) -> Option<FrameArray> {
    let mut raw = [FrameId::NULL; 3];
    if load_animation(&mut raw, keys, "animation") == 0 {
        free_image(&mut raw, "animation");
        return None;
    }
    let array = FrameArray::from_native(raw);
    array.observe("animation");
    Some(array)
}

impl RaceDesc {
    pub fn initialize_assets(&mut self, battle: bool) -> bool {
        let navigation = self.initialize_icons();
        let loaded = !battle || self.initialize_battle_assets();
        let data = &self.raw.ship_data;
        self.raw.assets = Some(Box::new(ShipAssets {
            navigation,
            captain: data.captain_control.background,
            victory: self.raw.loaded_victory.take(),
            sounds: self.raw.loaded_sounds.take(),
            animations: [data.special.clone(), data.weapon.clone(), data.ship.clone()],
        }));
        loaded
    }

    fn initialize_icons(&mut self) -> ShipNavigationAssets {
        let resources = &self.raw.resources;
        let icons = resources.icons.and_then(LoadedDrawable::load);
        let melee_icon = resources.melee_icon.and_then(LoadedDrawable::load);
        let strings = resources.strings.and_then(LoadedTextTable::load);
        let info = &mut self.raw.ship_info;
        info.icons = icon_frame(&icons);
        info.melee_icon = icon_frame(&melee_icon);
        info.race_strings = strings
            .as_ref()
            .map_or(std::ptr::null_mut(), |table| table.as_raw().cast());
        ShipNavigationAssets {
            melee_icon,
            icons,
            strings,
        }
    }

    fn initialize_battle_assets(&mut self) -> bool {
        let data = &mut self.raw.ship_data;
        let Some(ship) = load_animation_set(self.raw.resources.ship) else {
            return false;
        };
        data.ship = ship;
        for (keys, frames) in [
            (self.raw.resources.weapon, &mut data.weapon),
            (self.raw.resources.special, &mut data.special),
        ] {
            if keys[0].is_some() {
                let Some(loaded) = load_animation_set(keys) else {
                    return false;
                };
                *frames = loaded;
            }
        }
        self.initialize_battle_audio_and_captain()
    }

    fn initialize_battle_audio_and_captain(&mut self) -> bool {
        let data = &mut self.raw.ship_data;
        if let Some(key) = self.raw.resources.captain {
            data.captain_control.background = crate::battle::frames::load_graphic(key);
            if data.captain_control.background.is_null() {
                return false;
            }
        }
        if let Some(key) = self.raw.resources.victory {
            self.raw.loaded_victory = crate::sound::resource_types::LoadedMusic::load(key);
            data.victory_ditty = self
                .raw
                .loaded_victory
                .as_ref()
                .map_or(std::ptr::null_mut(), |music| music.identity().cast());
            if self.raw.loaded_victory.is_none() {
                return false;
            }
        }
        if let Some(key) = self.raw.resources.sounds {
            self.raw.loaded_sounds = crate::sound::owned_bank::LoadedSoundBank::load(key);
            data.ship_sounds = self
                .raw
                .loaded_sounds
                .as_ref()
                .map_or(std::ptr::null_mut(), |bank| bank.as_raw().cast());
            if self.raw.loaded_sounds.is_none() {
                return false;
            }
        }
        true
    }

    pub fn unload(mut self, world: &mut crate::World, battle: bool) {
        self.cleanup(world);
        crate::collections::observation::record(
            crate::collections::observation::Event::ResourcesFree {
                descriptor: self.address(),
                icons: true,
                battle,
            },
        );
        if let Some(assets) = self.raw.assets.take() {
            assets.release(battle);
        }
    }

    pub fn into_campaign_probe(mut self, world: &mut crate::World) -> CampaignProbeAssets {
        self.cleanup(world);
        crate::collections::observation::record(
            crate::collections::observation::Event::ResourcesFree {
                descriptor: self.address(),
                icons: false,
                battle: false,
            },
        );
        let assets = self
            .raw
            .assets
            .take()
            .unwrap_or_else(|| panic!("campaign probe requires loaded assets"));
        assert!(
            assets.animations.iter().all(FrameArray::is_absent)
                && assets.captain.is_null()
                && assets.victory.is_none()
                && assets.sounds.is_none(),
            "campaign probe must be loaded without battle resources"
        );
        CampaignProbeAssets {
            _navigation: assets.navigation,
        }
    }

    pub fn catalog_name(&self) -> &[u8] {
        let assets = self
            .raw
            .assets
            .as_ref()
            .unwrap_or_else(|| panic!("catalog descriptor has no loaded assets"));
        assets
            .navigation
            .strings
            .as_ref()
            .and_then(|table| table.text(2))
            .unwrap_or_else(|| panic!("catalog string table has no ship name"))
            .to_bytes()
    }
}

impl ShipAssets {
    fn release(self, battle: bool) {
        if battle {
            for array in &self.animations {
                let mut raw = array.views().map(|frame| frame.raw());
                free_image(&mut raw, "animation");
                array.retire_views();
            }
        }
        if battle {
            drop(LoadedDrawable::from_frame(self.captain));
            drop(self.victory);
            drop(self.sounds);
        }
        drop(self.navigation);
    }
}

use crate::battle::frames::{free_image, load_animation};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn campaign_owner_releases_melee_then_icons_and_keeps_retained_readers_alive() {
        use crate::collections::observation::{self, Event};
        observation::enable();
        observation::drain();
        let melee_icon = LoadedDrawable::create(2, 0, 0, 1).unwrap();
        let icons = LoadedDrawable::create(2, 0, 0, 2).unwrap();
        let melee = melee_icon.lease();
        let icon_frames = icons.lease();
        let allocated: Vec<_> = observation::drain()
            .into_iter()
            .filter_map(|event| match event {
                Event::Drawable(address) => Some(address),
                _ => None,
            })
            .collect();
        assert_eq!(allocated.len(), 2);
        let strings = LoadedTextTable::fixture(vec![b"probe\0".to_vec()]);
        let retained_text = strings.entry(0).unwrap();
        let mut descriptor = RaceDesc::empty();
        descriptor.raw.assets = Some(Box::new(ShipAssets {
            navigation: ShipNavigationAssets {
                melee_icon: Some(melee_icon),
                icons: Some(icons),
                strings: Some(strings),
            },
            captain: FrameId::NULL,
            victory: None,
            sounds: None,
            animations: std::array::from_fn(|_| FrameArray::EMPTY),
        }));
        let picker = descriptor.picker_resources();
        let assets = descriptor.raw.assets.take().unwrap();
        let mut world = crate::World::new();
        let mut other = crate::World::new();
        world.retain_campaign_probe(CampaignProbeAssets {
            _navigation: assets.navigation,
        });
        drop(descriptor);
        world.reset_game_data();
        other.release_campaign_probe();
        assert!(observation::drain().is_empty());
        world.release_campaign_probe();
        let released: Vec<_> = observation::drain()
            .into_iter()
            .filter_map(|event| match event {
                Event::DrawableFree(address) => Some(address),
                _ => None,
            })
            .collect();
        assert_eq!(released, allocated);
        assert_eq!(retained_text.current_label(), (c"probe", 5));
        assert_eq!(
            picker.strings.as_ref().unwrap().current_label(),
            (c"probe", 5)
        );
        assert!(melee.frame(0).is_some());
        assert!(icon_frames.frame(1).is_some());
        world.release_campaign_probe();
        assert!(observation::drain().is_empty());
    }

    #[test]
    fn descriptor_labels_borrow_the_owned_table_and_keep_character_bounds() {
        let mut descriptor = RaceDesc::empty();
        let table = LoadedTextTable::fixture(vec![b"Z\xc3\xb6q\0extra\0".to_vec(), vec![0]]);
        let pointer = table.text(0).unwrap().as_ptr();
        descriptor.raw.assets = Some(Box::new(ShipAssets {
            navigation: ShipNavigationAssets {
                icons: None,
                melee_icon: None,
                strings: Some(table),
            },
            captain: FrameId::NULL,
            victory: None,
            sounds: None,
            animations: std::array::from_fn(|_| FrameArray::EMPTY),
        }));
        let (text, count) = descriptor.raw.label(0);
        assert_eq!(text.to_bytes(), "Zöq".as_bytes());
        assert_eq!(count, 3);
        assert_eq!(text.as_ptr(), pointer);
        assert_eq!(descriptor.raw.label(2), (text, count));
        assert_eq!(descriptor.raw.label(1), (c"", 0));
    }

    #[test]
    fn early_captain_release_consumes_captured_authority_not_the_mutable_view() {
        let captain = FrameId::from_bits(1 << 16);
        let mut descriptor = RaceDesc::empty();
        descriptor.raw.assets = Some(Box::new(ShipAssets {
            navigation: ShipNavigationAssets {
                icons: None,
                melee_icon: None,
                strings: None,
            },
            captain,
            victory: None,
            sounds: None,
            animations: std::array::from_fn(|_| FrameArray::EMPTY),
        }));
        descriptor.raw.ship_data.captain_control.background = FrameId::from_bits(1 << 16 | 1);
        assert_eq!(descriptor.raw.take_captain_background(), captain);
        assert!(descriptor
            .raw
            .ship_data
            .captain_control
            .background
            .is_null());
        let assets = descriptor.raw.assets.as_ref().unwrap();
        assert!(assets.captain.is_null());
        assert!(assets.navigation.icons.is_none());
        assert!(assets.navigation.melee_icon.is_none());
        assert!(assets.navigation.strings.is_none());
        assert!(assets.victory.is_none());
        assert!(assets.sounds.is_none());
        assert!(assets.animations.iter().all(FrameArray::is_absent));
        assert!(descriptor.raw.take_captain_background().is_null());
    }
}
