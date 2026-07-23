use serde::ser::SerializeStruct;
use thiserror::Error;

use crate::model::validation::ValidationError;

/// Crate-wide error type. Serializes to a UI-friendly shape so Tauri commands
/// can return it directly to the frontend.
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("keychain error: {0}")]
    Keychain(String),

    #[error("validation failed ({} issue(s))", .0.len())]
    Validation(Vec<ValidationError>),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("could not determine the application config directory")]
    NoConfigDir,

    #[error("SSH error: {0}")]
    Ssh(String),

    #[error("authentication failed")]
    AuthFailed,

    #[error("host key rejected: {0}")]
    HostKeyRejected(String),

    #[error("{0}")]
    Unsupported(String),
}

impl From<russh::Error> for Error {
    fn from(err: russh::Error) -> Self {
        Error::Ssh(err.to_string())
    }
}

impl From<russh::keys::Error> for Error {
    fn from(err: russh::keys::Error) -> Self {
        Error::Ssh(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    fn kind(&self) -> &'static str {
        match self {
            Error::Io(_) => "io",
            Error::Serde(_) => "serde",
            Error::Keychain(_) => "keychain",
            Error::Validation(_) => "validation",
            Error::NotFound(_) => "notFound",
            Error::NoConfigDir => "noConfigDir",
            Error::Ssh(_) => "ssh",
            Error::AuthFailed => "authFailed",
            Error::HostKeyRejected(_) => "hostKeyRejected",
            Error::Unsupported(_) => "unsupported",
        }
    }
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut st = serializer.serialize_struct("Error", 3)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        let empty: Vec<ValidationError> = Vec::new();
        match self {
            Error::Validation(fields) => st.serialize_field("fields", fields)?,
            _ => st.serialize_field("fields", &empty)?,
        }
        st.end()
    }
}
