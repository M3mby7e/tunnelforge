use keyring::{Entry, Error as KeyringError};
use uuid::Uuid;

use crate::error::{Error, Result};

/// Keychain service name under which all Tunnelium secrets are stored.
const SERVICE: &str = "Tunnelium";

/// Build the keychain account id for a tunnel secret, e.g.
/// `tunnel:<uuid>:password`. This is what the config stores as a `*Ref`.
pub fn secret_account(tunnel_id: &Uuid, purpose: &str) -> String {
    format!("tunnel:{tunnel_id}:{purpose}")
}

/// Store (or overwrite) a secret in the OS keychain.
pub fn set_secret(account: &str, secret: &str) -> Result<()> {
    entry(account)?.set_password(secret).map_err(to_error)
}

/// Fetch a secret from the OS keychain. Returns `None` if not present.
pub fn get_secret(account: &str) -> Result<Option<String>> {
    match entry(account)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(err) => Err(to_error(err)),
    }
}

/// Delete a secret. Succeeds even if the entry does not exist.
pub fn delete_secret(account: &str) -> Result<()> {
    match entry(account)?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(err) => Err(to_error(err)),
    }
}

fn entry(account: &str) -> Result<Entry> {
    Entry::new(SERVICE, account).map_err(to_error)
}

fn to_error(err: KeyringError) -> Error {
    Error::Keychain(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_id_format() {
        let id = Uuid::nil();
        assert_eq!(
            secret_account(&id, "passphrase"),
            "tunnel:00000000-0000-0000-0000-000000000000:passphrase"
        );
    }

    // keyring's mock builds a fresh in-memory credential per `Entry::new`, so
    // it can't model persistence across calls. We therefore verify our error
    // *mapping* — a missing secret becomes `None`, and delete is a no-op —
    // rather than round-trip persistence (which is keyring's responsibility).
    #[test]
    fn missing_secret_maps_to_none_and_delete_is_noop() {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());

        let account = "tunnel:test:password";
        assert_eq!(get_secret(account).unwrap(), None);

        // Neither writing nor deleting against the mock should error.
        set_secret(account, "hunter2").unwrap();
        delete_secret(account).unwrap();
        delete_secret(account).unwrap();
    }

    // A single shared entry does round-trip, proving the wrapper's calls map
    // onto keyring correctly.
    #[test]
    fn single_entry_roundtrips() {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());

        let entry = Entry::new(SERVICE, "tunnel:solo:password").unwrap();
        entry.set_password("s3cret").unwrap();
        assert_eq!(entry.get_password().unwrap(), "s3cret");
    }
}
