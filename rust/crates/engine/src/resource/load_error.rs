//! Errors from resolving and acquiring assets, retained until the caller's policy decision.
use super::{AssetKind, Content, IndexSource, ResolveError, Resolved, Revision};
use std::{error::Error, ffi::CStr, path::PathBuf};

pub type Cause = Box<dyn Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
#[error("{path:?}: {source}")]
pub struct FileError {
    pub path: PathBuf,
    #[source]
    pub source: Cause,
}
impl FileError {
    pub fn at(path: impl Into<PathBuf>, source: impl Into<Cause>) -> Self {
        Self {
            path: path.into(),
            source: source.into(),
        }
    }
    pub fn invalid(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::at(
            path,
            std::io::Error::new(std::io::ErrorKind::InvalidData, message.into()),
        )
    }
}

#[derive(Clone, Debug)]
pub struct AssetSource {
    pub descriptor: String,
    pub index: IndexSource,
}

#[derive(Debug, thiserror::Error)]
#[error("cannot load {kind:?} {key} from {resolved:?}: {source}")]
pub struct AssetLoadError {
    pub key: String,
    pub kind: AssetKind,
    pub revision: Revision,
    pub resolved: Option<Box<AssetSource>>,
    #[source]
    pub source: Cause,
}

impl Content {
    /// Wrong kinds remain construction errors; missing definitions are acquisition failures.
    pub fn require(&self, key: &CStr, kind: AssetKind) -> Result<Resolved<'_>, AssetLoadError> {
        self.resolve(key, kind).map_err(|error| match error {
            ResolveError::WrongKind { .. } => panic!("{error}"),
            error => AssetLoadError {
                key: key.to_string_lossy().into_owned(),
                kind,
                revision: self.revision().clone(),
                resolved: None,
                source: Box::new(error),
            },
        })
    }
}

#[derive(Clone, Debug)]
pub struct AssetContext {
    pub key: String,
    pub kind: AssetKind,
    pub revision: Revision,
    pub resolved: AssetSource,
}
impl AssetContext {
    pub fn error(&self, source: impl Into<Cause>) -> AssetLoadError {
        AssetLoadError {
            key: self.key.clone(),
            kind: self.kind,
            revision: self.revision.clone(),
            resolved: Some(Box::new(self.resolved.clone())),
            source: source.into(),
        }
    }
}
impl Resolved<'_> {
    pub fn context(&self, key: &CStr) -> AssetContext {
        AssetContext {
            key: key.to_string_lossy().into_owned(),
            kind: self.kind,
            revision: self.revision.clone(),
            resolved: AssetSource {
                descriptor: self.descriptor.to_owned(),
                index: self.source.clone(),
            },
        }
    }
    pub fn load_error(&self, key: &CStr, source: impl Into<Cause>) -> AssetLoadError {
        self.context(key).error(source)
    }
}
