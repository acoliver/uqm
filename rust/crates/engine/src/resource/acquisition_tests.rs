use super::*;
use crate::{
    graphics::{drawable::LoadedDrawable, font::LoadedFont},
    host::Files,
    io::{fixture::TempMount, Placement},
    sound::{owned_bank::LoadedSoundBank, resource_types::LoadedMusic},
    strings::{
        conversation::ConversationTable,
        owned_table::{LoadedBinaryTable, LoadedTextTable},
    },
    video::legacy::LoadedVideo,
};
use std::{io, path::Path};

fn failure<T>(result: Result<T, AssetLoadError>) -> AssetLoadError {
    match result {
        Ok(_) => panic!("acquisition unexpectedly succeeded"),
        Err(error) => error,
    }
}

#[test]
#[serial_test::serial]
fn all_descriptor_families_retain_real_acquisition_causes_and_catalog_context() {
    let mount = TempMount::new();
    mount.write("assets.rmp", b"graphic = GFXRES:missing.ani\nfont = FONTRES:missing.font\ntext = STRTAB:missing.txt\nbinary = BINTAB:missing.bin\nconversation = CONVERSATION:missing.phrases:voice/\nsound = SNDRES:missing.snd\nmusic = MUSICRES:missing.ogg\nvideo = 3DOVID:missing.duk\n");
    let content = mount.content(c"assets.rmp");
    let errors = [
        failure(LoadedDrawable::load(&content, c"graphic")),
        failure(LoadedFont::load(&content, c"font")),
        failure(LoadedTextTable::load(&content, c"text")),
        failure(LoadedBinaryTable::load(&content, c"binary")),
        failure(ConversationTable::load(&content, c"conversation")),
        failure(LoadedSoundBank::load(&content, c"sound")),
        failure(LoadedMusic::load(&content, c"music")),
        failure(LoadedVideo::load(&content, c"video").and_then(|video| video.play())),
    ];
    for error in errors {
        assert_eq!(&error.revision, content.revision());
        let key = std::ffi::CString::new(error.key.as_str()).unwrap();
        let resolved = content.resolve(&key, error.kind).unwrap();
        let source = error.resolved.as_ref().unwrap();
        assert_eq!(source.descriptor, resolved.descriptor);
        assert_eq!(source.index.index.as_c_str(), c"assets.rmp");
        assert_eq!(
            source
                .index
                .directory
                .read(Path::new("assets.rmp"))
                .unwrap(),
            mount.dir().read(Path::new("assets.rmp")).unwrap()
        );
        let file = error.source.downcast_ref::<FileError>().unwrap();
        let cause = file.source.downcast_ref::<io::Error>().unwrap();
        assert_eq!(cause.kind(), io::ErrorKind::NotFound, "{error}");
    }
}

#[test]
#[serial_test::serial]
fn required_navigation_preserves_open_cause_and_retries_actual_acquisition() {
    let mount = TempMount::new();
    mount.write(
        "navigation.rmp",
        b"graphics.flagshipstatus = GFXRES:status.ani\ngraphics.miscdata = GFXRES:misc.ani\n",
    );
    mount.write("status.ani", b"pixel.png -1 0 0 0\n");
    mount.write("pixel.png", &pixel());
    let content = mount.content(c"navigation.rmp");
    let mut kernel = crate::Kernel::new();
    kernel.admit_content(content.clone());
    for _ in 0..2 {
        let error = kernel
            .load_navigation_graphics(Default::default())
            .unwrap_err();
        assert_eq!(error.resource, "graphics.miscdata");
        assert_eq!(error.source.revision, *content.revision());
        assert_eq!(
            error.source.resolved.as_ref().unwrap().descriptor,
            "misc.ani"
        );
        let file = error.source.source.downcast_ref::<FileError>().unwrap();
        let io = file.source.downcast_ref::<io::Error>().unwrap();
        let open = io
            .get_ref()
            .unwrap()
            .downcast_ref::<crate::io::vfs::OpenError>()
            .unwrap();
        assert_eq!(open.virtual_path, Path::new("/misc.ani"));
        assert_eq!(open.mount_path, mount.path_of(""));
        assert_eq!(open.member, Path::new("misc.ani"));
        assert_eq!(open.source.kind(), io::ErrorKind::NotFound);
        assert!(kernel.flag_stat_frame.is_null());
        assert!(kernel.misc_data_frame.is_null());
        assert!(kernel.radar_context.is_none());
    }
    mount.write("misc.ani", b"pixel.png -1 0 0 0\n");
    kernel.load_navigation_graphics(Default::default()).unwrap();
    assert!(!kernel.misc_data_frame.is_null());
    assert!(kernel.font_grad_frame.is_null());
    kernel.release_navigation_graphics();
}

fn pixel() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[1, 2, 3, 255])
            .unwrap();
    }
    bytes
}

#[test]
#[serial_test::serial]
fn corrupt_shadow_keeps_decoder_cause_and_never_retries_lower_image() {
    let lower = TempMount::new();
    lower.write(
        "assets.rmp",
        b"graphic = GFXRES:art.ani\nmusic = MUSICRES:bad.wav\n",
    );
    lower.write("art.ani", b"pixel.png -1 0 0 0\n");
    lower.write("pixel.png", &pixel());
    let upper = TempMount::new();
    upper.write("pixel.png", b"corrupt PNG");
    lower
        .vfs()
        .mount_dir("/", &upper.path_of(""), Placement::Top, true);
    let content = lower.content(c"assets.rmp");
    let graphic = failure(LoadedDrawable::load(&content, c"graphic"));
    let file = graphic.source.downcast_ref::<FileError>().unwrap();
    assert_eq!(file.path, Path::new("pixel.png"));
    assert!(file.source.downcast_ref::<png::DecodingError>().is_some());
    lower.write("bad.wav", b"corrupt WAV");
    let music = failure(LoadedMusic::load(&content, c"music"));
    let file = music.source.downcast_ref::<FileError>().unwrap();
    assert!(file
        .source
        .downcast_ref::<crate::sound::decoder::DecodeError>()
        .is_some());
}

#[test]
fn missing_definition_has_revision_and_wrong_kind_remains_an_invariant() {
    let mount = TempMount::new();
    mount.write("assets.rmp", b"wrong = INT32:42\n");
    let content = mount.content(c"assets.rmp");
    let error = failure(LoadedTextTable::load(&content, c"missing"));
    assert_eq!(error.revision, *content.revision());
    assert!(error.resolved.is_none());
    assert!(
        matches!(error.source.downcast_ref::<ResolveError>(), Some(ResolveError::Missing { key }) if key == "missing")
    );
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| LoadedTextTable::load(
            &content, c"wrong"
        )))
        .is_err()
    );
}

#[test]
#[serial_test::serial]
fn malformed_package_and_movie_keep_their_decoder_types() {
    let mount = TempMount::new();
    mount.write(
        "assets.rmp",
        b"font = FONTRES:bad.font\nvideo = 3DOVID:bad.duk\nsound = SNDRES:sounds/bank.snd\n",
    );
    mount.write("bad.font", b"not a ZIP package");
    for extension in ["duk", "frm", "hdr", "tbl"] {
        mount.write(&format!("bad.{extension}"), b"invalid");
    }
    mount.write("sounds/bank.snd", b"missing.wav\n");
    let content = mount.content(c"assets.rmp");
    let font = failure(LoadedFont::load(&content, c"font"));
    let file = font.source.downcast_ref::<FileError>().unwrap();
    let io = file.source.downcast_ref::<io::Error>().unwrap();
    let archive = io
        .get_ref()
        .unwrap()
        .downcast_ref::<crate::io::zip_reader::ArchiveError>()
        .unwrap();
    assert_eq!(archive.path, Path::new("bad.font"));
    assert!(archive
        .source
        .downcast_ref::<zip::result::ZipError>()
        .is_some());
    let video = failure(LoadedVideo::load(&content, c"video").and_then(|video| video.play()));
    let file = video.source.downcast_ref::<FileError>().unwrap();
    assert!(file
        .source
        .downcast_ref::<crate::video::VideoError>()
        .is_some());
    let bank = failure(LoadedSoundBank::load(&content, c"sound"));
    assert_eq!(
        bank.resolved.as_ref().unwrap().descriptor,
        "sounds/bank.snd"
    );
    let file = bank.source.downcast_ref::<FileError>().unwrap();
    assert_eq!(file.path, Path::new("sounds/missing.wav"));
    assert_eq!(
        file.source.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::NotFound
    );
}
