use std::collections::BTreeSet;
use std::env;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "build/navigation.rs"]
mod navigation;

/// One C source per line, relative to the repository root. Deleting a ported
/// C file means deleting it and its line here.
const C_SOURCE_LIST: &str = "build/c-sources.txt";
const SDL_PACKAGE: [&str; 1] = ["sdl2"];
const REPOSITORY_INCLUDE_ROOTS: [&str; 3] = ["sc2", "sc2/src", "sc2/src/libs/uio"];
const COMPILE_FLAGS: [&str; 3] = ["-std=gnu99", "-O2", "-fPIC"];

/// Preprocessor defines every C object is compiled with.
const C_DEFINES: [&str; 36] = [
    "GFXMODULE_SDL",
    "HAVE_JOYSTICK",
    "HAVE_ZIP=1",
    "NETPLAY=NETPLAY_FULL",
    "OVCODEC_NONE",
    "RUST_OWNS_MAIN",
    "SDL_DIR=SDL2",
    "THREADLIB_SDL",
    "USE_INTERNAL_MIKMOD",
    "USE_PLATFORM_ACCEL",
    "USE_RUST_AIFF",
    "USE_RUST_AUDIO",
    "USE_RUST_AUDIO_HEART",
    "USE_RUST_BRIDGE",
    "USE_RUST_CLOCK",
    "USE_RUST_COMM",
    "USE_RUST_DUKAUD",
    "USE_RUST_FILE",
    "USE_RUST_GFX",
    "USE_RUST_INPUT",
    "USE_RUST_MAINLOOP=1",
    "USE_RUST_MEM",
    "USE_RUST_MIXER",
    "USE_RUST_MOD",
    "USE_RUST_OGG",
    "USE_RUST_RESOURCE",
    "USE_RUST_RESTART",
    "USE_RUST_SHIPS",
    "USE_RUST_STATE",
    "USE_RUST_SUPERMELEE",
    "USE_RUST_THREADS",
    "USE_RUST_UIO",
    "USE_RUST_VIDEO",
    "USE_RUST_VIDPLAYER",
    "USE_RUST_WAV",
    "_THREAD_SAFE",
];

/// Libraries and frameworks the game links besides SDL, per target OS.
struct PlatformLinks {
    packages: &'static [&'static str],
    libraries: &'static [&'static str],
    frameworks: &'static [&'static str],
}

fn platform_links(target_os: &str) -> Result<PlatformLinks, String> {
    match target_os {
        "macos" => Ok(PlatformLinks {
            packages: &["liblzma", "bzip2"],
            libraries: &["sandbox", "z", "m", "objc"],
            frameworks: &["Cocoa", "CoreAudio", "AudioToolbox", "CoreFoundation"],
        }),
        "linux" => Ok(PlatformLinks {
            packages: &["liblzma"],
            libraries: &["z", "m", "bz2"],
            frameworks: &[],
        }),
        other => Err(format!("unsupported target OS: {other}")),
    }
}

fn main() {
    if let Err(error) = run() {
        fail(error);
    }
}

fn run() -> Result<(), String> {
    generate_state_bindings(Path::new("../sc2/src/uqm/globdata.h"));
    generate_hash_abi_bindings()?;
    generate_input_abi_bindings()?;
    generate_global_state_abi_bindings()?;
    navigation::generate()?;
    generate_ship_abi_bindings()?;
    generate_netplay_abi_bindings()?;
    generate_graphics_abi_bindings()?;
    let mut packages = discover_packages(&SDL_PACKAGE)?;
    let target_os = env_value("CARGO_CFG_TARGET_OS")?;
    let links = platform_links(&target_os)?;
    if env::var_os("CARGO_FEATURE_LINKED_C_ARCHIVE").is_some() {
        packages.extend(discover_packages(links.packages)?);
        build_c_archive(&target_os, &packages)?;
    } else {
        emit_package_links(&packages);
    }
    emit_platform_links(&links);
    Ok(())
}

fn fail(message: impl Display) -> ! {
    eprintln!("build failed: {message}");
    std::process::exit(1)
}

struct NativePackages {
    include_paths: BTreeSet<PathBuf>,
    link_paths: BTreeSet<PathBuf>,
    link_files: BTreeSet<PathBuf>,
    libraries: BTreeSet<String>,
    framework_paths: BTreeSet<PathBuf>,
    frameworks: BTreeSet<String>,
}

impl NativePackages {
    fn extend(&mut self, other: Self) {
        self.include_paths.extend(other.include_paths);
        self.link_paths.extend(other.link_paths);
        self.link_files.extend(other.link_files);
        self.libraries.extend(other.libraries);
        self.framework_paths.extend(other.framework_paths);
        self.frameworks.extend(other.frameworks);
    }
}

fn discover_packages(packages: &[&str]) -> Result<NativePackages, String> {
    let mut result = NativePackages {
        include_paths: BTreeSet::new(),
        link_paths: BTreeSet::new(),
        link_files: BTreeSet::new(),
        libraries: BTreeSet::new(),
        framework_paths: BTreeSet::new(),
        frameworks: BTreeSet::new(),
    };
    for package in packages {
        let library = pkg_config::Config::new()
            .cargo_metadata(false)
            .probe(package)
            .map_err(|error| format!("pkg-config discovery failed for {package}: {error}"))?;
        result.include_paths.extend(library.include_paths);
        result.link_paths.extend(library.link_paths);
        result.link_files.extend(library.link_files);
        result.libraries.extend(library.libs);
        result.framework_paths.extend(library.framework_paths);
        result.frameworks.extend(library.frameworks);
    }
    Ok(result)
}

/// Compile every listed C source and archive the objects into `libuqm_c.a`.
fn build_c_archive(target_os: &str, packages: &NativePackages) -> Result<(), String> {
    let rust_root = PathBuf::from(env_value("CARGO_MANIFEST_DIR")?);
    let repo_root = rust_root
        .parent()
        .ok_or_else(|| "Rust crate has no repository parent".to_string())?
        .to_path_buf();
    let list_path = rust_root.join(C_SOURCE_LIST);
    println!("cargo:rerun-if-changed={}", list_path.display());
    // Headers are not tracked individually; any change under sc2/src rebuilds.
    println!(
        "cargo:rerun-if-changed={}",
        repo_root.join("sc2/src").display()
    );
    let list = fs::read_to_string(&list_path)
        .map_err(|error| format!("cannot read {}: {error}", list_path.display()))?;
    let sources: Vec<&str> = list
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    let out_dir = output_directory()?;
    let object_dir = out_dir.join("native");
    fs::create_dir_all(&object_dir)
        .map_err(|error| format!("cannot create {}: {error}", object_dir.display()))?;

    let compiler = env::var("CC").unwrap_or_else(|_| "cc".into());
    let mut common: Vec<String> = C_DEFINES
        .iter()
        .map(|define| format!("-D{define}"))
        .collect();
    if env::var_os("CARGO_FEATURE_DEBUG_PROCESS").is_some() {
        common.push("-DDEBUG".into());
    }
    for include in &packages.include_paths {
        common.extend(["-I".into(), include.to_string_lossy().into_owned()]);
    }
    for root in REPOSITORY_INCLUDE_ROOTS {
        common.extend([
            "-I".into(),
            repo_root.join(root).to_string_lossy().into_owned(),
        ]);
    }
    common.extend(COMPILE_FLAGS.map(String::from));

    let mut objects = Vec::with_capacity(sources.len());
    for source in &sources {
        let object = object_dir.join(format!("{}.o", source.replace('/', "__")));
        run_status(
            Command::new(&compiler)
                .args(&common)
                .arg("-c")
                .arg(repo_root.join(source))
                .arg("-o")
                .arg(&object),
            &format!("compile {source}"),
        )?;
        objects.push(object);
    }

    let archive = out_dir.join("libuqm_c.a");
    match fs::remove_file(&archive) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "cannot remove stale {}: {error}",
                archive.display()
            ))
        }
    }
    run_status(
        Command::new(env::var("AR").unwrap_or_else(|_| "ar".into()))
            .arg("rcs")
            .arg(&archive)
            .args(&objects),
        "archive creation",
    )?;
    emit_archive_link(target_os, &out_dir, &archive)?;
    emit_package_links(packages);
    Ok(())
}

fn emit_archive_link(target_os: &str, out_dir: &Path, archive: &Path) -> Result<(), String> {
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    match target_os {
        "macos" => {
            // The library records no #[link] for the C archive so pure test
            // builds stay C-free; the game binary force-loads the archive
            // itself to resolve the C registration symbols.
            println!(
                "cargo:rustc-link-arg-bin=uqm=-Wl,-force_load,{}",
                archive.display()
            );
        }
        "linux" => {
            println!("cargo:rustc-link-lib=static:+whole-archive,-bundle=uqm_c");
        }
        other => return Err(format!("unsupported target OS: {other}")),
    }
    Ok(())
}

fn emit_package_links(packages: &NativePackages) {
    for path in &packages.link_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }
    for path in &packages.framework_paths {
        println!("cargo:rustc-link-search=framework={}", path.display());
    }
    for path in &packages.link_files {
        println!("cargo:rustc-link-arg-bin=uqm={}", path.display());
    }
    for library in &packages.libraries {
        println!("cargo:rustc-link-lib={library}");
    }
    for framework in &packages.frameworks {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}

fn emit_platform_links(links: &PlatformLinks) {
    for library in links.libraries {
        println!("cargo:rustc-link-lib={library}");
    }
    for framework in links.frameworks {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}

fn require<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => fail(format!("{context}: {error}")),
    }
}

fn require_some<T>(value: Option<T>, context: &str) -> T {
    match value {
        Some(value) => value,
        None => fail(context),
    }
}

/// Bindgen's default formatter shells out to `rustfmt`.
///
/// Generated bindings are written to `OUT_DIR`, included by the crate and never
/// read as source, so formatting them changes nothing a human or a gate looks
/// at: `cargo fmt --check` does not reach `OUT_DIR`. What it does change is what
/// the build needs. `rustfmt` is a toolchain component, and the pinned CI
/// bootstrap installs the controller toolchain with `--profile minimal`, which
/// deliberately excludes it. Bindgen treats the missing binary as non-fatal,
/// prints `Failed to run rustfmt: ... (non-fatal, continuing)` and emits
/// unformatted output instead: a silent fallback that makes the generated bytes
/// depend on whatever happens to be installed on the machine. Selecting no
/// formatter removes the dependency and the fallback together.
const ABI_BINDING_FORMATTER: bindgen::Formatter = bindgen::Formatter::None;
fn generate_graphics_abi_bindings() -> Result<(), String> {
    let headers = [
        "../sc2/src/libs/graphics/gfxintrn.h",
        "../sc2/src/libs/graphics/tfb_prim.h",
        "../sc2/src/libs/graphics/drawcmd.h",
        "../sc2/src/libs/graphics/dcqueue.h",
        "../sc2/src/libs/graphics/widgets.h",
        "../sc2/src/options.h",
    ];
    let mut builder = bindgen::Builder::default();
    for header in headers {
        println!("cargo:rerun-if-changed={header}");
        builder = builder.header(header);
    }
    if env::var_os("CARGO_FEATURE_DEBUG_PROCESS").is_some() {
        builder = builder.clang_arg("-DDEBUG");
    }
    let bindings = builder
        .clang_arg("-I../sc2").clang_arg("-I../sc2/src")
        .allowlist_type("(frame_desc|drawable_desc|font_desc|FontPage|tfb_char|context_desc|PRIMITIVE|RESOURCE_DATA|TFB_DrawCommand.*|WIDGET.*|CHOICE_OPTION)")
        .allowlist_function("(TFB_.*|BoxIntersect|AllocDrawable|CreateDrawable|FixContextFontEffect|SetContextForeGroundColor|GetContextValidRect|BatchGraphics|UnbatchGraphics|LoadResourceFromPath|InstallResTypeVectors|res_.*|LengthResFile|GetStringAddress|uio_(fgets|ftell|fseek|fread|fclose|fopen|openDir|mountDir|closeDir|unmountDir|openDirRelative)|.*DirEntry.*|getCharFromString)")
        .allowlist_var("(WIDGET_EVENT_.*|TFB_DRAWCOMMANDTYPE_.*|_CurFramePtr|_CurFontPtr|_pCurContext|_locPrim|_GraphicsStatusFlags|_cur_resfile_name|contentDir|repository|QuitPosted|.*DRAWABLE|WANT_PIXMAP|MAPPED_TO_DISPLAY|FBK_DIRTY|TFB_SCREEN_MAIN|.*PRIM|NUM_PRIMS|END_OF_LIST|BATCH_BUILD_PAGE|DRAW_REPLACE_MODE|ALIGN_.*|MAX_DELTAS|CHARACTER_PAGE_MASK|uio_FSTYPE_ZIP|uio_MOUNT_RDONLY|uio_MOUNT_TOP|match_MATCH_SUBSTRING)")
        .blocklist_function("(uio_.*|TFB_DrawImage_New|TFB_DrawCanvas_Rescale_Nearest)")
        .generate_comments(false).layout_tests(true).formatter(ABI_BINDING_FORMATTER)
        .generate().map_err(|e| format!("cannot generate graphics ABI: {e}"))?;
    fs::write(
        output_directory()?.join("graphics_abi.rs"),
        bindings.to_string(),
    )
    .map_err(|e| format!("cannot write graphics ABI: {e}"))
}

/// Live race ABI, including the member alignments required by RACE_DESC.
fn generate_ship_abi_bindings() -> Result<(), String> {
    let bindings = bindgen::Builder::default()
        .clang_arg("-DMELEESETUP_INTERNAL").clang_arg("-DPICKMELE_INTERNAL")
        .header("../sc2/src/uqm/supermelee/melee.h")
        .header("../sc2/src/uqm/supermelee/buildpick.h").header("../sc2/src/uqm/flash.h").header("../sc2/src/uqm/supermelee/netplay/netmisc.h").header("../sc2/src/uqm/igfxres.h")
        .header("../sc2/src/uqm/ships/ship.h")
        .header("../sc2/src/uqm/ship.h")
        .header("../sc2/src/uqm/build.h")
        .header("../sc2/src/uqm/collide.h")
        .header("../sc2/src/uqm/master.h")
        .header("../sc2/src/uqm/settings.h")
        .header("../sc2/src/uqm/setup.h")
        .header("../sc2/src/uqm/init.h")
        .header("../sc2/src/uqm/status.h")
        .header("../sc2/src/uqm/process.h")
        .header("../sc2/src/options.h")
        .header("../sc2/src/libs/graphics/gfx_common.h")
        .header("../sc2/src/uqm/globdata.h")
        .header("../sc2/src/libs/mathlib.h")
        .header("../sc2/src/uqm/controls.h")
        .header("../sc2/src/uqm/hyper.h")
        .clang_arg("-I../sc2")
        .clang_arg("-I../sc2/src")
        .clang_arg("-DNETPLAY=NETPLAY_FULL")
        .header("../sc2/src/uqm/battle.h")
        .header("../sc2/src/uqm/battlecontrols.h")
        .header("../sc2/src/uqm/tactrans.h")
        .header("../sc2/src/uqm/encount.h")
        .header("../sc2/src/uqm/intel.h")
        .header("../sc2/src/uqm/sounds.h")
        .header("../sc2/src/uqm/sis.h")
        .header("../sc2/src/uqm/colors.h")
        .header("../sc2/src/uqm/gamestr.h")
        .header("../sc2/src/uqm/menustat.h")
        .header("../sc2/src/uqm/gameopt.h")
        .header("../sc2/src/uqm/pickship.h")
        .header("../sc2/src/uqm/cons_res.h")
        .header("../sc2/src/uqm/resinst.h")
        .header("../sc2/src/uqm/nameref.h")
        .header("../sc2/src/uqm/supermelee/netplay/netmelee.h")
        .header("../sc2/src/uqm/supermelee/netplay/notifyall.h")
        .header("../sc2/src/libs/graphics/drawable.h")
        .header("../sc2/src/libs/graphics/drawcmd.h")
        .header("../sc2/src/libs/async.h")
        .header("../sc2/src/libs/misc.h")
        .allowlist_function("(ANGLE_TO_FACING|ARCTAN|AddEscortShips|AllocDisplayPrim|AllocElement|AllocLink|AnyButtonPress|Async_process|Avoid|BUILD_COLOR|BatchGraphics|Battle|BattleInputBuffer_pop|BattleInputBuffer_push|BattleSong|Build|BuildBattle|CALC_ZOOM_STUFF|COSINE|CalcDisplayCoord|CalcReduction|CalcSoundPosition|CalcView|CalculateEscortsWorth|CalculateGravity|CaptainsWindow|CaptureDrawable|CaptureSound|CheckAlliance|ClearDrawable|ClearPlayerInput|CloneShipFragment|CollidingElement|CollisionPossible|CountEscortShips|CountLinks|CurrentInputToBattleInput|DISPLAY_ALIGN|DISPLAY_ALIGN_X|DISPLAY_ALIGN_Y|DISPLAY_TO_WORLD|DecFrameIndex|DeltaCrew|DeltaEnergy|DeltaSISGauges|DeltaStatistics|DeltaVelocityComponents|DestroyDrawable|DestroyMusic|DestroySound|DittyPlaying|DoBattle|DoInput|DoMenuChooser|DoRunAway|DoSelectAction|DrawArmadaPickShip|DrawBatch|DrawBattleCrewAmount|DrawCaptainsWindow|DrawCrewFuelString|DrawFadeText|DrawFilledRectangle|DrawFilledStamp|DrawHyperCoords|DrawMenuStateStrings|DrawRectangle|DrawSISFrame|DrawSISMessage|DrawSISTitle|DrawStamp|DrawStatusMessage|DrawablesIntersect|EncounterBattle|Entice|EscortFeasibilityStudy|FACING_TO_ANGLE|FindAliveStarShip|FleetIsInfinite|FlushInput|FlushSounds|FreeBattleSong|FreeDisplayPrim|FreeElement|FreeLink|FreeShipFrag|GAME_STRING|GET_GAME_STATE|GLOBAL|GLOBAL_SIS|GRAVITY_MASS|GameOptions|GetClusterName|GetCurrentVelocityComponents|GetElementStarShip|GetEncounterStarShip|GetEscortByStarShipIndex|GetFrameHot|GetFrameIndex|GetFrameRect|GetHeadElement|GetHeadLink|GetIndexFromStarShip|GetInitialMeleeStarShips|GetInitialStarShips|GetLinkSize|GetNextStarShip|GetNextVelocityComponents|GetPlayerOrder|GetPredLink|GetPrimLinks|GetPrimType|GetRaceKnownSize|GetShipFragQueueForPlayer|GetStarShipFromIndex|GetStringAddress|GetSuccElement|GetSuccLink|GetTimeCounter|GetVelocityTravelAngle|GetWinnerStarShip|HIBYTE|HIWORD|HaveEscortShip|IncFrameIndex|InitCyborg|InitDisplayList|InitEncounter|InitIntersectEndPoint|InitIntersectFrame|InitIntersectStartPoint|InitSISContexts|InitShipStatus|InitShips|InitStatusOffsets|InsertElement|InsertPrim|InsertQueue|IsVelocityZero|JournalInput|LOBYTE|LOWORD|LoadGraphic|LoadMusic|LoadSound|LockElement|LockFleetInfo|LockLink|LockShipFrag|LockStarShip|MAKE_RGB15|MAKE_RGB15_INIT|MAKE_WORD|MANEUVERABILITY|MAX_ALLOWED_SPEED_SQR|MI|MIN_DITTY_FRAME_COUNT|MakeLinks|MeleeGameOver|MeleeShipDeath|MoveGalaxy|MoveSIS|NORMALIZE_ANGLE|NORMALIZE_FACING|NUM_DISPLAY_PTS|NUM_FADES|NUM_SHIP_FADES|NameCaptain|Netplay_NotifyAll_battleInput|Netplay_NotifyAll_checksum|Netplay_Notify_frameCount|Netplay_isLocalReady|Netplay_localReady|OBJECT_CLOAKED|ORG_JUMP_X|ORG_JUMP_Y|OpponentAlive|PLRPlaying|PickCaptainName|PlayDitty|PlayMusic|PlaySound|PlotIntercept|PostProcess|PostProcessQueue|PostProcessStatus|PreProcess|PreProcessQueue|PreProcessStatus|ProcessCollisions|ProcessInput|ProcessSound|Pursue|PutElement|PutQueue|RecordShipDeath|RedrawQueue|ReinitQueue|ReleaseDrawable|ReleaseSound|RemoveElement|RemoveEscortShips|RemoveQueue|RemoveSomeEscortShips|RemoveSoundsForObject|ResetWinnerStarShip|RunAwayAllowed|SET_GAME_STATE|SINE|ScreenTransition|SetAbsFrameIndex|SetAbsSoundIndex|SetContext|SetContextBackGroundColor|SetContextFont|SetContextForeGroundColor|SetElementStarShip|SetEquFrameIndex|SetEscortCrewComplement|SetFlashRect|SetGraphicScale|SetGraphicScaleMode|SetMenuSounds|SetPlayerInput|SetPrimColor|SetPrimLinks|SetPrimType|SetRaceAllied|SetRelFrameIndex|SetStatusMessageMode|SetTransitionSource|SetUpElement|SetVelocityComponents|SetVelocityVector|SetWinnerStarShip|SleepThreadUntil|StartShipExplosion|StartSphereTracking|StopAllBattleMusic|StopDitty|StopMusic|StopSound|TFB_DrawScreen_SetMipmap|TFB_Random|TFB_SeedRandom|TaskSwitch|ThrustShip|TimeSpaceMatterConflict|TurnShip|UnbatchGraphics|UninitEncounter|UninitShips|UnlockElement|UnlockFleetInfo|UnlockLink|UnlockShipFrag|UnlockStarShip|Untarget|UpdateShipFragCrew|UpdateSoundPositions|VELOCITY_TO_WORLD|VelocitySquared|WEAPON_RANGE|WORLD_TO_DISPLAY|WORLD_TO_TURN|WORLD_TO_VELOCITY|WRAP_DELTA_X|WRAP_DELTA_Y|WRAP_X|WRAP_Y|WaitForAnyButton|WaitForSoundEnd|ZOOM_JUMP|ZeroVelocityComponents|_GetPredLink|_GetSuccLink|addLocalChecksum|animation_preprocess|battleEndReady|battleEndReadyComputer|battleEndReadyHuman|battleEndReadyNetwork|checkOtherShipLifeSpan|cleanup_dead_ship|collide|collision|crc_finish|crc_init|crc_processState|cycle_ion_trail|do_damage|elementsOfSamePlayer|explode|explosion_preprocess|flee_preprocess|flushPacketQueue|flushPacketQueues|font_DrawText|frameInput|frameInputHuman|frame_cb|free_gravity_well|free_ship|getBattleInputBuffer|getBattleInputDelay|getNumNetConnections|hyper_transition|inHQSpace|inHyperSpace|inQuasiSpace|inertial_thrust|initBattleInputBuffers|initBattleStateDataConnections|initChecksumBuffers|intelligence_func|load_gravity_well|load_ship|log_add|negotiateReady|negotiateReadyConnections|netInput|new_ship|preprocess_dead_ship|readyForBattleEnd|readyForBattleEndPlayer|readyToEnd2Callback|readyToEndCallback|resetConnections|rust_automation_seed_value|rust_battle_frame_advance|rust_battle_frame_reset|rust_ships_build|rust_ships_clone_fragment|rust_ships_death|rust_ships_postprocess|rust_ships_preprocess|rust_ships_spawn|selectAllShips|setBattleStateConnections|setMinShipLifeSpan|setMinStarShipLifeSpan|setupBattleInputOrder|ship_death|ship_intelligence|ship_movement|ship_postprocess|ship_preprocess|ship_transition|ship_weapons|spawn_ion_trail|spawn_ship|square_root|strupr|tactical_intelligence|uninitBattleInputBuffers|uninitChecksumBuffers|utf8StringCopy|verifyChecksums|waitReady|waitResetConnections)")
        .allowlist_var(".*")
        .allowlist_function("(printf|sprintf|GetShipBox|DrawMeleeShipStrings|Melee_flashSelection|DoMelee|DrawMeleeIcon|RepairMeleeFrame|Melee_LocalChange_team|LoadDirEntryTable|ConfirmSaveLoad|Flash_(createHighlight|setRect)|PulsedInputToBattleInput|ConfirmExit|SetDefaultMenuRepeatDelay|FadeScreen|FlushColorXForms|allConnected|setStateConnections)")
        .allowlist_type("(RACE_DESC|STARSHIP|ELEMENT|EVALUATE_DESC|MISSILE_BLOCK|LASER_BLOCK|PRIMITIVE|gfx_object|QUEUE|LINK|SHIP_BASE|SHIP_FRAGMENT|FLEET_INFO|SHIP_INFO|CAPTAIN_STUFF|STAMP|POINT|RECT|TEXT|Color|STATUS_FLAGS|SPECIES_ID|BYTE|COUNT|BOOLEAN|SIZE|HELEMENT|HLINK|HSHIPFRAG|FRAME|COORD|VELOCITY_DESC)")
        .allowlist_function("(initialize_missile|initialize_laser|TrackShip|ship_intelligence|ship_weapons|DeltaCrew|DeltaEnergy|AllocElement|SetVelocityComponents|GetFrameRect|SetAbsFrameIndex|SetAbsStringTableIndex|ProcessSound|SetVelocityVector|GetCurrentVelocityComponents|GetNextVelocityComponents|DeltaVelocityComponents|SetEquFrameIndex|IncFrameIndex|DecFrameIndex|weapon_collision|collision|inertial_thrust|Untarget|AbandonShip|PlotIntercept|ARCTAN|square_root|TFB_Random|TFB_SeedRandom|GetFrameCount|RemoveElement|FreeElement|ModifySilhouette|CalculateGravity|TimeSpaceMatterConflict|inHQSpace|HyperspaceMenu|Build|GetEscortByStarShipIndex|NameCaptain|CloneShipFragment|GetIndexFromStarShip|AddEscortShips|CalculateEscortsWorth|SetRaceAllied|StartSphereTracking|CountEscortShips|HaveEscortShip|EscortFeasibilityStudy|CheckAlliance|RemoveSomeEscortShips|RemoveEscortShips|SetEscortCrewComplement|GetNextStarShip|GetInitialStarShips|ship_preprocess|ship_postprocess|animation_preprocess|GetHeadLink|GetLinkSize|AllocLink|LockLink|UnlockLink|PutQueue|RemoveQueue|InsertQueue|CountLinks|LockShipFrag|UnlockShipFrag|FreeShipFrag|LockFleetInfo|UnlockFleetInfo|LockStarShip|UnlockStarShip|GetEncounterStarShip|GetPlayerOrder|GetInitialMeleeStarShips|PickCaptainName|DeltaSISGauges|DeltaStatistics|DrawFilledRectangle|DrawRectangle|DrawCrewFuelString|SetContextForeGroundColor|SetContextFont|font_DrawText|BatchGraphics|UnbatchGraphics|SetPrimType|InsertElement|GetHeadElement|LockElement|UnlockElement|SetElementStarShip|GetElementStarShip|SetVelocityVector|GetCurrentVelocityComponents|DeltaVelocityComponents|VelocitySquared|GetVelocityTravelAngle|hyper_transition|ship_transition|spawn_ion_trail|OpponentAlive|BattleSong|PLRPlaying|InitIntersectStartPoint|InitIntersectEndPoint|InitStatusOffsets|PreProcessStatus|PostProcessStatus|DrawCaptainsWindow|DrawBattleCrewAmount|SetContext|DrawStamp|InitShipStatus|SetAbsSoundIndex|SetRelFrameIndex)")
        .allowlist_var("(disp_q|GlobData|sinetab|ScreenWidth|ScreenHeight|PLAYER_SHIP|APPEARING|DISAPPEARING|NONSOLID|FINITE_LIFE|CHANGING|IGNORE_SIMILAR|SEEKING_WEAPON|SEEKING_SPECIAL|POINT_DEFENSE|FIRES_FORE|FIRES_AFT|DONT_CHASE|IMMEDIATE_WEAPON|CREW_OBJECT|CREW_IMMUNE|LEFT|RIGHT|THRUST|WEAPON|SPECIAL|SHIP_AT_MAX_SPEED|SHIP_BEYOND_MAX_SPEED|SHIP_IN_GRAVITY_WELL|MAX_SHIP_MASS|MAX_ENERGY_SIZE|FIRES_RIGHT|FIRES_LEFT|MEDIUM_SHIP|NEUTRAL_PLAYER_NUM|COLLISION|COLLISION_THRUST_WAIT|DEFY_PHYSICS|POST_PROCESS|PRE_PROCESS|SHIELD_DEFENSE|HYPERJUMP_LIFE|ENEMY_SHIP_INDEX|ENEMY_WEAPON_INDEX|CREW_OBJECT_INDEX|ONE_SHIFT|NORMAL_LIFE|STAMP_PRIM|STAMPFILL_PRIM|IN_ENCOUNTER|PROBE_ILWRATH_ENCOUNTER|END_PROBE_ILWRATH_ENCOUNTER|SPHERE_RADIUS_INCREMENT|INFINITE_RADIUS|MAX_CREW_SIZE|CLOSE_RANGE_WEAPON|LONG_RANGE_WEAPON|SLOW_SHIP|FULL_CIRCLE|HALF_CIRCLE|QUADRANT|OCTANT|FACING_SHIFT|CIRCLE_SHIFT|SIN_SHIFT|VELOCITY_SHIFT|MODIFY_IMAGE|RPG_PLAYER_NUM|NPC_PLAYER_NUM|COLLISION_TURN_WAIT|IN_BATTLE|IN_HYPERSPACE|SUPER_MELEE|PLAYER_CAPTAIN|NUM_MODULE_SLOTS|NUM_DRIVE_SLOTS|NUM_JET_SLOTS|CREW_POD_CAPACITY|FUSION_THRUSTER|TURNING_JETS|CREW_POD|TRACKING_SYSTEM|ANTIMISSILE_DEFENSE|SHIVA_FURNACE|DYNAMO_UNIT|GUN_WEAPON|CANNON_WEAPON|PulsedInputState|KEY_MENU_CANCEL)")
        .allowlist_type("(MASTER_SHIP_INFO|ComputerInputContext|SoundPosition|GETMELEE_STATE|BattleStateData|MELEE_STATE|MeleeSetup|MeleeTeam)")
        .allowlist_function("(tactical_intelligence|CurrentInputToBattleInput|SleepThread|ChannelPlaying|GetPositionalObject|SetPositionalObject|UpdateSoundPosition|PlaySoundEffect|GetStringAddress|InitQueue|UninitQueue|GetStarShipFromIndex|ReleaseDrawable|DestroyDrawable|ReleaseStringTable|DestroyStringTable|DrawablesIntersect|SetContext|DrawStamp|InitShipStatus|animation_preprocess|AllocDisplayPrim|InsertPrim|inHQSpace)")
        .allowlist_var("(PlayerControl|IN_LAST_BATTLE|IN_ENCOUNTER|SUPER_MELEE|CYBORG_CONTROL|PSYTRON_CONTROL|RPG_PLAYER_NUM|NPC_PLAYER_NUM|BATTLE_ESCAPE|BATTLE_WEAPON|ONE_SECOND|DisplayArray|FIRST_SFX_CHANNEL|LAST_SFX_CHANNEL|MIN_FX_CHANNEL|NUM_FX_CHANNELS|GAME_SOUND_PRIORITY|stars_in_space|status_y_offsets|StatusContext|VIEW_.*|TFB_SCALE_STEP|optMeleeScale|MAX_REDUCTION|MAX_VIS_REDUCTION|BACKGROUND_SHIFT|ZOOM_SHIFT|MAX_ZOOM_OUT|MAX_TIME_VALUE|NO_PRIM|MODIFY_SWAP|TARGET_DAMAGED_.*|GRAB_CREW|GOOD_GUY|BAD_GUY|DEAD_GUY|SAMATRA_SHIP|MAX_BUILT_SHIPS|UNDEFINED_DELTA|LEFT|RIGHT|THRUST|WEAPON|SPECIAL|SHIP_AT_MAX_SPEED|SHIP_BEYOND_MAX_SPEED|SHIP_IN_GRAVITY_WELL|LOW_ON_ENERGY|APPEARING|CHANGING|PLAYER_SHIP|IGNORE_SIMILAR|FINITE_LIFE|COLLISION|NORMAL_LIFE|STAMP_PRIM|NUM_PLAYERS|STATUS_WIDTH|STATUS_HEIGHT|SHIP_STATUS_HEIGHT|SHIP_INFO_HEIGHT|CAPTAIN_XOFFS|CAPTAIN_YOFFS|CAPTAIN_WIDTH|CAPTAIN_HEIGHT|BATTLE_CREW_X|BATTLE_CREW_Y|MAX_CREW_SIZE|PLAYER_CAPTAIN|MAKE_RGB15|BLACK_COLOR|UNDEFINED_COLOR|GameSounds|GRAVITY_MASS|NUM_CAPTAINS_NAMES|NAME_OFFSET)")
        .allowlist_function("NetConnection_(getStateData|getDiscriminant|getState|setState)")
        // native::abi reuses Rust exports or typed conversions over existing imports.
        // A second extern declaration would give shared symbols incompatible pointer types.
        .blocklist_function(
            "(AbandonShip|AllocElement|AllocLink|CalcSoundPosition|CalculateGravity\
             |CaptureDrawable|CurrentInputToBattleInput|DecFrameIndex|DestroyDrawable\
             |DestroyMusic|DestroySound|DestroyStringTable|DoInput|DoMenuChooser|DrawBatch\
             |DrawFilledRectangle|DrawFilledStamp|DrawRectangle|DrawSISTitle\
             |DrawStamp|DrawablesIntersect|FleetIsInfinite|FreeElement|GetFrameCount\
             |GetFrameHot|GetFrameIndex|GetFrameRect|GetStringAddress|IncFrameIndex\
             |InsertQueue|ModifySilhouette|MoveGalaxy|PLRPlaying|PlayMusic|PlaySound\
             |ProcessSound|PutQueue|ReleaseDrawable|ReleaseStringTable\
             |RemoveSoundsForObject|ScreenTransition|SetAbsFrameIndex\
             |SetAbsStringTableIndex|SetContext|SetContextFont|SetEquFrameIndex\
             |SetFlashRect|SetRelFrameIndex|SetStatusMessageMode|SetTransitionSource\
             |SetVelocityVector|SleepThread|TFB_DrawScreen_SetMipmap|TFB_SeedRandom\
             |TrackShip|battleEndReadyComputer|battleEndReadyHuman\
             |battleEndReadyNetwork|collide|do_damage|font_DrawText|frameInputHuman\
             |inHQSpace|inHyperSpace|initialize_laser|initialize_missile\
             |utf8StringCopy|weapon_collision)",
        )
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate_comments(false)
        .layout_tests(true)
        .formatter(ABI_BINDING_FORMATTER)
        .generate()
        .map_err(|error| format!("cannot generate ship ABI bindings: {error}"))?;
    fs::write(
        output_directory()?.join("ship_abi.rs"),
        bindings.to_string(),
    )
    .map_err(|error| format!("cannot write generated ship ABI bindings: {error}"))
}

fn generate_hash_abi_bindings() -> Result<(), String> {
    let out_dir = output_directory()?;
    let char_header = "../sc2/src/libs/uio/charhashtable.h";
    let string_header = "../sc2/src/libs/strings/stringhashtable.h";
    for header in [
        char_header,
        string_header,
        "../sc2/src/libs/uio/hashtable.h",
    ] {
        println!("cargo:rerun-if-changed={header}");
    }
    let bindings = bindgen::Builder::default()
        .header(char_header)
        .header(string_header)
        .clang_arg("-I../sc2")
        .clang_arg("-I../sc2/src")
        .allowlist_type("(Char|String)HashTable_.*")
        .allowlist_function("(Char|String)HashTable_.*")
        .generate_comments(false)
        .layout_tests(false)
        .formatter(ABI_BINDING_FORMATTER)
        .generate()
        .map_err(|error| format!("cannot generate retained hash-table ABI bindings: {error}"))?;
    let generated = canonicalize_hash_abi_names(bindings.to_string())?;
    fs::write(out_dir.join("hash_table_abi.rs"), generated)
        .map_err(|error| format!("cannot write generated hash-table ABI bindings: {error}"))
}
fn generate_input_abi_bindings() -> Result<(), String> {
    let out_dir = output_directory()?;
    let header = "../sc2/src/uqm/controls.h";
    println!("cargo:rerun-if-changed={header}");
    let bindings = bindgen::Builder::default()
        .header(header)
        .clang_arg("-I../sc2")
        .clang_arg("-I../sc2/src")
        .allowlist_type("CONTROLLER_INPUT_STATE")
        .allowlist_type("CONTROL_TEMPLATE")
        .allowlist_var("(NUM_KEYS|NUM_MENU_KEYS|NUM_TEMPLATES)")
        .allowlist_var("(CurrentInputState|PulsedInputState)")
        .generate_comments(false)
        .formatter(ABI_BINDING_FORMATTER)
        .generate()
        .map_err(|error| format!("cannot generate controller-input ABI bindings: {error}"))?;
    fs::write(
        out_dir.join("controller_input_abi.rs"),
        bindings.to_string(),
    )
    .map_err(|error| format!("cannot write generated controller-input ABI bindings: {error}"))
}

/// Bindgen layout of the live game-state record the Save/Load runtime reads
/// and writes: `GAME_STATE`, `GLOBDATA`, `CLOCK_STATE`, `EVENT`,
/// `VELOCITY_DESC`, the `FLEET_INFO`, `SHIP_FRAGMENT`, `IP_GROUP` and
/// `ENCOUNTER` queue elements (with `BRIEF_SHIP_INFO`), the `NUM_EVENTS` pool
/// size and the `MAX_BATTLE_GROUPS`, `MAX_ENCOUNTERS` and `MAX_HYPER_SHIPS`
/// bounds. The solar-system state (`SOLARSYS_STATE`, `PLANET_DESC`) the
/// Save/Load summary reads comes from `planets.h`, with the game-string bases
/// of `gamestr.h` and the star-index enum of `gendef.h` that name its worlds.
/// `FRAME_DESC` comes from `drawable.h`, where it is defined: every other
/// header only forward-declares it, which bindgen renders as an empty struct.
/// The image and parent drawable it points at are never dereferenced from
/// Rust, so they stay opaque rather than pulling the graphics driver types in.
///
/// `ELEMENT` (element.h) and `STARSHIP` (races.h) are the display-list element
/// and the ship record the save of a hyperspace flagship follows; the race
/// descriptor a `STARSHIP` points at is never dereferenced from Rust, so it
/// stays opaque with the drawables.
///
/// Bindgen's layout tests are on: each one compares a generated type's size,
/// alignment and member offsets with what clang computed from the header, which
/// is the check that a Rust read of a C struct (such as `FRAME_DESC.Index`)
/// lands on the field C means.
///
/// The hand-written mirrors in `comm/locdata.rs` and `time/clock_bridge.rs`
/// compare every offset, size and alignment against these at compile time, so
/// a header change that moves a field stops the build instead of corrupting a
/// save.
fn generate_global_state_abi_bindings() -> Result<(), String> {
    let out_dir = output_directory()?;
    let globdata = "../sc2/src/uqm/globdata.h";
    let events = "../sc2/src/uqm/gameev.h";
    let races = "../sc2/src/uqm/races.h";
    let groups = "../sc2/src/uqm/grpinfo.h";
    let encounters = "../sc2/src/uqm/encount.h";
    let planets = "../sc2/src/uqm/planets/planets.h";
    let game_strings = "../sc2/src/uqm/gamestr.h";
    let star_indexes = "../sc2/src/uqm/gendef.h";
    let generate = "../sc2/src/uqm/planets/generate.h";
    let drawable = "../sc2/src/libs/graphics/drawable.h";
    let ship_build = "../sc2/src/uqm/build.h";
    for header in [
        globdata,
        events,
        races,
        groups,
        encounters,
        planets,
        game_strings,
        star_indexes,
        generate,
        drawable,
        ship_build,
    ] {
        println!("cargo:rerun-if-changed={header}");
    }
    let bindings = bindgen::Builder::default()
        .header(globdata)
        .header(events)
        .header(races)
        .header(groups)
        .header(encounters)
        .header(planets)
        .header(game_strings)
        .header(star_indexes)
        .header(generate)
        .header(drawable)
        .header(ship_build)
        .header("../sc2/src/uqm/save.h")
        .allowlist_function(
            "(planetOuterLocation|XFormIPLoc|EscortFeasibilityStudy|_clip_line|DrawBatch\
             |DrawLine|DrawOval|DrawFilledOval|GetContextForeGroundColor\
             |GetContextFGFrame|GetContextClipRect|GetContextDrawMode)",
        )
        .clang_arg("-I../sc2")
        .clang_arg("-I../sc2/src")
        .allowlist_type(
            "(SUMMARY_DESC|LOCDATA|ANIMATION_DESC|GAME_STATE|GLOBDATA|CLOCK_STATE|EVENT|VELOCITY_DESC|FLEET_INFO|SHIP_FRAGMENT\
             |IP_GROUP|ENCOUNTER|BRIEF_SHIP_INFO|SOLARSYS_STATE|PLANET_DESC|STAR_DESC|FRAME_DESC|ELEMENT\
             |STARSHIP|GenerateFunctions|gfx_object|MENU_STATE)",
        )
        .opaque_type("(tfb_image|drawable_desc|race_desc|RACE_DESC)")
        .allowlist_var(
            "(NUM_EVENTS|MAX_BATTLE_GROUPS|MAX_ENCOUNTERS|MAX_HYPER_SHIPS|MAX_SUNS|MAX_PLANETS\
             |MAX_MOONS|STAR_NUMBER_BASE|PLANET_NUMBER_BASE|STARBASE_STRING_BASE|SOL_DEFINED|SPATHI_DEFINED\
             |SAMATRA_DEFINED|CHMMR_DEFINED|ILWRATH_DEFINED|PLAYER_SHIP|RPG_PLAYER_NUM|EncounterRace|EncounterGroup\
             |ScreenWidth|ScreenHeight)",
        )
        .generate_comments(false)
        .layout_tests(true)
        .formatter(ABI_BINDING_FORMATTER)
        .generate()
        .map_err(|error| format!("cannot generate global-state ABI bindings: {error}"))?;
    fs::write(out_dir.join("global_state_abi.rs"), bindings.to_string())
        .map_err(|error| format!("cannot write generated global-state ABI bindings: {error}"))
}

/// Rename the generated hash-table ABI to this crate's spelling, and restore
/// the C symbol each renamed function still has to link against.
///
/// The renaming rewrites `CharHashTable_newHashTable` to
/// `CharHashTablenewHashTable`, which is no longer the symbol the archive
/// exports, so every renamed function needs an explicit `#[link_name]` putting
/// the original back. That injection anchors on `pub fn <name>`, which both
/// bindgen's own token rendering and any formatter emit identically; it must
/// never anchor on indentation, because an anchor that stops matching would
/// leave the renamed function pointing at a symbol nothing defines and the
/// failure would only surface as an undefined symbol at link time. An
/// injection that matches nothing is therefore an error here rather than a
/// silent no-op.
fn canonicalize_hash_abi_names(mut bindings: String) -> Result<String, String> {
    let replacements = [
        ("CharHashTable_", "CharHashTable"),
        ("StringHashTable_", "StringHashTable"),
        ("STRING_TABLE_ENTRY_DESC", "StringTableEntryDesc"),
        ("string_table_entry", "StringTableEntry"),
        ("uio_bool", "UioBool"),
        ("uio_uint32", "UioUint32"),
        ("hashFunction", "hash_function"),
        ("equalFunction", "equal_function"),
        ("copyFunction", "copy_function"),
        ("freeKeyFunction", "free_key_function"),
        ("freeValueFunction", "free_value_function"),
        ("minFillQuotient", "min_fill_quotient"),
        ("maxFillQuotient", "max_fill_quotient"),
        ("initialSize", "initial_size"),
        ("minSize", "min_size"),
        ("maxSize", "max_size"),
        ("hashMask", "hash_mask"),
        ("numEntries", "num_entries"),
        ("numCollisions", "num_collisions"),
        ("hashTable", "hash_table"),
        ("bucketNr", "bucket_nr"),
    ];
    for (original, canonical) in replacements {
        bindings = bindings.replace(original, canonical);
    }
    for family in ["CharHashTable", "StringHashTable"] {
        for function in [
            "newHashTable",
            "add",
            "remove",
            "find",
            "count",
            "deleteHashTable",
            "getIterator",
            "iteratorDone",
            "iteratorKey",
            "iteratorValue",
            "iteratorNext",
            "freeIterator",
        ] {
            let declaration = format!("pub fn {family}{function} ");
            let linked = format!("#[link_name = \"{family}_{function}\"] {declaration}");
            let occurrences = bindings.matches(&declaration).count();
            if occurrences != 1 {
                return Err(format!(
                    "expected one generated declaration for {family}_{function}, found {occurrences}; nearby declarations: {}",
                    bindings
                        .lines()
                        .filter(|line| line.contains(&format!("pub fn {family}")))
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
            }
            bindings = bindings.replace(&declaration, &linked);
        }
    }
    Ok(bindings)
}

fn run_status(command: &mut Command, label: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("cannot execute {label}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with {status}"))
    }
}

fn output_directory() -> Result<PathBuf, String> {
    env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "OUT_DIR is unavailable".into())
}

fn env_value(name: &str) -> Result<String, String> {
    env::var(name).map_err(|error| format!("{name} is unavailable: {error}"))
}

fn generate_state_bindings(globdata_path: &Path) {
    println!("cargo:rerun-if-changed={}", globdata_path.display());

    let source = require(
        fs::read_to_string(globdata_path),
        "failed to read sc2/src/uqm/globdata.h for Rust state bindings",
    );

    let mut bit = 0usize;
    let mut entries = Vec::new();

    for line in source.lines() {
        let Some((name, width)) = parse_game_state_entry(line) else {
            continue;
        };

        let start = bit;
        let end = bit + width - 1;
        entries.push((name, start, end));
        bit += width;
    }

    let num_bits = bit;
    let num_bytes = (num_bits + 7) >> 3;
    let out_dir = PathBuf::from(require_some(env::var_os("OUT_DIR"), "OUT_DIR not set"));
    let mut generated = String::new();

    generated.push_str("// @generated by rust/build.rs from sc2/src/uqm/globdata.h\n");
    generated.push_str("// Do not edit by hand.\n\n");
    generated.push_str(&format!(
        "pub const NUM_GAME_STATE_BITS: usize = {num_bits};\n"
    ));
    generated.push_str(&format!(
        "pub const NUM_GAME_STATE_BYTES: usize = {num_bytes};\n\n"
    ));
    generated.push_str("pub fn lookup_game_state_bits(name: &str) -> Option<(usize, usize)> {\n");
    generated.push_str("    match name {\n");

    for (name, start, end) in entries {
        generated.push_str(&format!("        \"{name}\" => Some(({start}, {end})),\n"));
    }

    generated.push_str("        _ => None,\n");
    generated.push_str("    }\n");
    generated.push_str("}\n");

    require(
        fs::write(out_dir.join("state_generated.rs"), generated),
        "failed to write generated Rust state bindings",
    );
}

fn parse_game_state_entry(line: &str) -> Option<(String, usize)> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') || !trimmed.contains("ADD_GAME_STATE") {
        return None;
    }

    let open = trimmed.find('(')?;
    let close = trimmed[open + 1..].find(')')? + open + 1;
    let inner = &trimmed[open + 1..close];
    let mut parts = inner.split(',').map(str::trim);

    let name = parts.next()?;
    let width = parts.next()?.parse().ok()?;

    Some((name.to_string(), width))
}

fn generate_netplay_abi_bindings() -> Result<(), String> {
    let bindings = bindgen::Builder::default()
        .header("../sc2/src/uqmversion.h")
        .header("../sc2/src/uqm/supermelee/netplay/packet.h")
        .header_contents("netplay_constants.h", r#"
#include "uqm/supermelee/netplay/netmisc.h"
#include "uqm/supermelee/meleeship.h"
enum UqmNetplayConstants {
    UQM_NETPLAY_BATTLE_FRAME_RATE = BATTLE_FRAME_RATE,
    UQM_NETPLAY_NUM_MELEE_SHIPS = NUM_MELEE_SHIPS
};
"#)
        .header("../sc2/src/uqm/supermelee/netplay/netmelee.h")
        .header("../sc2/src/uqm/supermelee/netplay/netmisc.h")
        .header("../sc2/src/uqm/supermelee/netplay/netsend.h")
        .header("../sc2/src/uqm/supermelee/netplay/notifyall.h")
        .header("../sc2/src/libs/async.h")
        .header("../sc2/src/libs/callback.h")
        .header("../sc2/src/libs/alarm.h")
        .header("../sc2/src/uqm/supermelee/netplay/proto/ready.h")
        .header("../sc2/src/uqm/supermelee/netplay/proto/reset.h")
        .header("../sc2/src/uqm/supermelee/netplay/proto/npconfirm.h")
        .header("../sc2/src/uqm/supermelee/melee.h")
        .header("../sc2/src/uqm/supermelee/pickmele.h")
        .header("../sc2/src/libs/mathlib.h")
        .clang_arg("-I../sc2")
        .clang_arg("-I../sc2/src")
        .clang_arg("-DNETPLAY=NETPLAY_FULL")
        .clang_arg("-DNETCONNECTION_INTERNAL")
        .allowlist_type("(ForEachConnectionCallback|NetworkInputContext|ConnectStateData|NetStateData|NetplayOptions|NetConnection|BattleStateData|ChecksumBuffer|crc_State|ELEMENT|STATE|Packet.*|UqmNetplayConstants)")
        .allowlist_function("(DoInput|MeleeSetup_.*|Melee_bootstrapSyncTeam|NetMelee_.*|closeFeedback|errorFeedback|connectedFeedback|flushPacketQueues|NetDescriptor_.*|NetManager_.*|Socket_.*|ListenState_.*|ConnectState_.*|flushPacketQueue|listenPort|connectHostByName|Callback_.*|Alarm_.*|Async_.*|NetConnection_.*|send.*|getBattleInput.*|BattleInputBuffer_push|Netplay_.*|resetFeedback|abortFeedback|confirmationCancelled|Melee_RemoteChange_.*|updateRandomSeed|updateMeleeSelection|TFB_SeedRandom)")
        .allowlist_var("(netConnections|packetTypeData|disp_q|battleFrameCount|NETPLAY_.*|BATTLE_FRAME_RATE|MELEE_FLEET_SIZE|MAX_TEAM_CHARS|NUM_MELEE_SHIPS|UQM_.*_VERSION)")
        .blocklist_function("NetConnection_(getStateData|getDiscriminant|getState|setState|getPlayerNr)")
        .raw_line("pub use super::netconnection::NetConnection_setState;")
        .generate_comments(false)
        .layout_tests(true)
        .formatter(ABI_BINDING_FORMATTER)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .map_err(|error| format!("cannot generate netplay ABI: {error}"))?;
    fs::write(
        output_directory()?.join("netplay_abi.rs"),
        bindings.to_string(),
    )
    .map_err(|error| format!("cannot write netplay ABI: {error}"))
}
