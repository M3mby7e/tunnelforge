use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How to authenticate to the SSH server.
///
/// Secrets are never stored inline — `*_ref` fields hold an OS-keychain account
/// id, and the actual value lives in the keychain (see `store::keychain_store`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AuthConfig {
    /// Account password (keychain ref).
    #[serde(rename_all = "camelCase")]
    Password { secret_ref: String },

    /// Private key file on disk, optionally unlocked by a passphrase.
    #[serde(rename_all = "camelCase")]
    PrivateKey {
        key_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        passphrase_ref: Option<String>,
    },

    /// Private key material imported into the keychain.
    #[serde(rename_all = "camelCase")]
    PrivateKeyInline {
        key_ref: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        passphrase_ref: Option<String>,
    },

    /// Use the running SSH agent.
    Agent,

    /// Server-driven prompts (e.g. 2FA / OTP).
    #[serde(rename_all = "camelCase")]
    KeyboardInteractive {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        secret_ref: Option<String>,
    },
}
