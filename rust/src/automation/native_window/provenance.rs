//! Command paths must bind to the retained inputs, never the source checkout.
use super::{valid_bound_command_path, NativeRetainedInput, NativeWindowProofError};
use std::path::Path;

pub(super) fn validate_command_paths(
    command: &[String],
    executable: &NativeRetainedInput,
    script: &NativeRetainedInput,
    content_parent: &Path,
) -> Result<(), NativeWindowProofError> {
    let command_paths = [
        command[0].as_str(),
        command[1]
            .strip_prefix("--configdir=")
            .ok_or(NativeWindowProofError::Receipt)?,
        command[2]
            .strip_prefix("--contentdir=")
            .ok_or(NativeWindowProofError::Receipt)?,
        command[3]
            .strip_prefix("--automation-script=")
            .ok_or(NativeWindowProofError::Receipt)?,
        command[4]
            .strip_prefix("--automation-output=")
            .ok_or(NativeWindowProofError::Receipt)?,
        command[5]
            .strip_prefix("--native-window-proof=")
            .ok_or(NativeWindowProofError::Receipt)?,
    ];
    let mut recorded_root = Path::new(command_paths[0]);
    for _ in Path::new(&executable.relative_path).components() {
        recorded_root = recorded_root
            .parent()
            .ok_or(NativeWindowProofError::Receipt)?;
    }
    if command_paths
        .iter()
        .any(|path| !valid_bound_command_path(path))
        || Path::new(command_paths[0]) != recorded_root.join(&executable.relative_path)
        || Path::new(command_paths[1]) != recorded_root.join("config")
        || Path::new(command_paths[2]) != recorded_root.join(content_parent)
        || Path::new(command_paths[3]) != recorded_root.join(&script.relative_path)
        || Path::new(command_paths[4]) != recorded_root.join("runtime-automation")
        || Path::new(command_paths[5]) != recorded_root.join("native-window-proof.json")
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(())
}
