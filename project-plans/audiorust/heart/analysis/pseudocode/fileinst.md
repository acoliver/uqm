# Pseudocode — `sound::fileinst`

Plan ID: `PLAN-20260225-AUDIO-HEART`

---

## 1. RAII Guard for cur_resfile_name

```
01: STRUCT FileLoadGuard<'a> {
02:   state: &'a Mutex<FileInstState>,
03: }
04:
05: IMPL Drop FOR FileLoadGuard {
06:   FUNCTION drop(&mut self)
07:     LET state = self.state.lock()
08:     SET state.cur_resfile_name = None   // REQ-FILEINST-LOAD-07
09:   END FUNCTION
10: }
11:
12: FUNCTION acquire_load_guard(filename) -> AudioResult<FileLoadGuard>
13:   LET state = FILE_STATE.lock()
14:   IF state.cur_resfile_name.is_some() THEN
15:     RETURN Err(AudioError::ConcurrentLoad)
16:   END IF
17:   SET state.cur_resfile_name = Some(filename.to_string())
18:   RETURN Ok(FileLoadGuard { state: &FILE_STATE })
```

Validation: REQ-FILEINST-LOAD-07
Side effects: Ensures cleanup on all exit paths (success, error, panic)

## 2. load_sound_file

```
20: FUNCTION load_sound_file(filename) -> AudioResult<SoundBank>
21:   // REQ-FILEINST-LOAD-01: concurrent guard check
22:   LET _guard = acquire_load_guard(filename)?
23:
24:   // Read resource file data
25:   LET data = read_resource_file(filename)
26:   IF data.is_err() THEN
27:     RETURN Err(AudioError::IoError(format!("failed to read: {}", filename)))
28:     // REQ-FILEINST-LOAD-03
29:   END IF
30:   LET data = data.unwrap()
31:
32:   // REQ-FILEINST-LOAD-02: delegate to get_sound_bank_data
33:   LET bank = get_sound_bank_data(filename, &data)?
34:
35:   // _guard dropped here → cur_resfile_name cleared
36:   RETURN Ok(bank)
```

Validation: REQ-FILEINST-LOAD-01..03
Error handling: ConcurrentLoad if already loading; IoError on read failure
Integration: Calls get_sound_bank_data (from sfx module)

## 3. load_music_file

```
40: FUNCTION load_music_file(filename) -> AudioResult<MusicRef>
41:   // REQ-FILEINST-LOAD-04: concurrent guard check
42:   LET _guard = acquire_load_guard(filename)?
43:
44:   // REQ-FILEINST-LOAD-05: validate filename
45:   LET validated = check_music_res_name(filename)
46:   IF validated.is_none() THEN
47:     RETURN Err(AudioError::ResourceNotFound(filename.to_string()))
48:   END IF
49:
50:   // Delegate to get_music_data
51:   LET music_ref = get_music_data(filename)?
52:
53:   // _guard dropped here → cur_resfile_name cleared
54:   RETURN Ok(music_ref)
55:
56:   // On error: _guard drop ensures cleanup   // REQ-FILEINST-LOAD-06
```

Validation: REQ-FILEINST-LOAD-04..06
Error handling: ConcurrentLoad if already loading; IoError on read failure
Integration: Calls check_music_res_name, get_music_data (from music module)

## 4. destroy_sound / destroy_music (delegating functions)

```
60: FUNCTION destroy_sound(bank) -> AudioResult<()>
61:   CALL release_sound_bank_data(bank)   // delegates to sfx module
62:   // REQ-SFX-RELEASE-04
63:
64: FUNCTION destroy_music(music_ref) -> AudioResult<()>
65:   CALL release_music_data(music_ref)   // delegates to music module
66:   // REQ-MUSIC-RELEASE-04
```

Validation: REQ-SFX-RELEASE-04, REQ-MUSIC-RELEASE-04
