//! `dosToUnixPath`: backslashes to slashes on a byte string.

use crate::io::paths::unc;

const BACKSLASH: u8 = b'\\';
const SLASH: u8 = b'/';

/// `path` with every `\` replaced by `/`, except that a leading UNC
/// `\\server\share` prefix keeps its leading backslashes and gets a backslash
/// between the server and the share, so that the prefix is the name the
/// platform stats. The bytes are not interpreted any further.
pub fn dos_to_unix_path(path: &[u8]) -> Vec<u8> {
    let prefix_len = unc::skip(path);
    let mut converted = Vec::with_capacity(path.len());
    converted.extend_from_slice(&path[..prefix_len]);
    if let Some(at) = converted.iter().skip(2).position(|&byte| byte == SLASH) {
        converted[2 + at] = BACKSLASH;
    }
    converted.extend(
        path[prefix_len..]
            .iter()
            .map(|&byte| if byte == BACKSLASH { SLASH } else { byte }),
    );
    converted
}
