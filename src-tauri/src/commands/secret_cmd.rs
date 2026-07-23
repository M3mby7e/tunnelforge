use crate::error::Error;
use crate::store::keychain_store;

/// Store a secret (password or key passphrase) in the OS keychain.
///
/// `account` is the keychain id referenced by a tunnel's `*Ref` field. The
/// frontend passes the secret in once; it is never read back out to the UI.
#[tauri::command]
pub fn set_secret(account: String, secret: String) -> Result<(), Error> {
    keychain_store::set_secret(&account, &secret)
}

/// Remove a secret from the OS keychain. No-op if it does not exist.
#[tauri::command]
pub fn clear_secret(account: String) -> Result<(), Error> {
    keychain_store::delete_secret(&account)
}
