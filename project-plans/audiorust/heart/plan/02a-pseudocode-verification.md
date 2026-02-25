# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P02a`

## Prerequisites
- Required: Phase P02 completed
- Expected files: 7 files in `analysis/pseudocode/`

## Verification Checklist

### Structural
- [ ] `analysis/pseudocode/stream.md` exists (> 200 lines)
- [ ] `analysis/pseudocode/trackplayer.md` exists (> 150 lines)
- [ ] `analysis/pseudocode/music.md` exists (> 80 lines)
- [ ] `analysis/pseudocode/sfx.md` exists (> 80 lines)
- [ ] `analysis/pseudocode/control.md` exists (> 60 lines)
- [ ] `analysis/pseudocode/fileinst.md` exists (> 30 lines)
- [ ] `analysis/pseudocode/heart_ffi.md` exists (> 100 lines)
- [ ] All algorithms use numbered lines

### Semantic — Completeness
- [ ] stream.md covers: init/uninit, create/destroy sample, play/stop/pause/resume/seek stream, decoder task, process_source_stream, process_music_fade, set_music_stream_fade, graph_foreground_stream, scope helpers, tag helpers
- [ ] trackplayer.md covers: splice_track, splice_multi_track, split_sub_pages, get_time_stamps, play/stop/jump/pause/resume track, seek_track, find_next/prev_page, TrackCallbacks impl, do_track_tag, position/subtitle queries
- [ ] music.md covers: plr_play_song, plr_stop, plr_playing, plr_seek, plr_pause, plr_resume, snd_play/stop_speech, get/release_music_data, check_music_res_name, set_music_volume, fade_music
- [ ] sfx.md covers: play/stop_channel, channel_playing, set_channel_volume, check_finished_channels, update_sound_position, get/set_positional_object, get_sound_bank_data, release_sound_bank_data
- [ ] control.md covers: SoundSourceArray::new, VolumeState::new, init/uninit_sound, stop/clean_source, stop_sound, set_sfx/speech_volume, sound_playing, wait_for_sound_end
- [ ] fileinst.md covers: FileLoadGuard, load_sound_file, load_music_file, destroy_sound, destroy_music
- [ ] heart_ffi.md covers: all 60+ FFI functions, C callback wrapper

### Semantic — Quality
- [ ] Error handling paths documented for every function that returns Result
- [ ] Mutex acquisition explicitly stated
- [ ] Integration boundaries (mixer calls, decoder calls) marked
- [ ] Side effects noted

## Gate Decision
- [ ] PASS: proceed to P03
- [ ] FAIL: revise pseudocode
