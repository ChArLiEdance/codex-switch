use crate::profile::TargetEnvironment;
use keyring::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const SERVICE_NAME: &str = "codex-switch";

pub trait SecretStore {
    fn put_secret(&self, key: &str, value: &str) -> Result<(), SecretStoreError>;
    fn get_secret(&self, key: &str) -> Result<Option<String>, SecretStoreError>;
    fn delete_secret(&self, key: &str) -> Result<(), SecretStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretStoreError {
    Backend(String),
    LockPoisoned,
}

pub struct KeychainSecretStore {
    service: String,
}

impl KeychainSecretStore {
    pub fn new() -> Self {
        Self {
            service: SERVICE_NAME.to_string(),
        }
    }

    fn entry(&self, key: &str) -> Result<Entry, SecretStoreError> {
        Entry::new(&self.service, key).map_err(|error| SecretStoreError::Backend(error.to_string()))
    }
}

impl Default for KeychainSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for KeychainSecretStore {
    fn put_secret(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        self.entry(key)?
            .set_password(value)
            .map_err(|error| SecretStoreError::Backend(error.to_string()))
    }

    fn get_secret(&self, key: &str) -> Result<Option<String>, SecretStoreError> {
        match self.entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(SecretStoreError::Backend(error.to_string())),
        }
    }

    fn delete_secret(&self, key: &str) -> Result<(), SecretStoreError> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(SecretStoreError::Backend(error.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretEnvelope {
    pub key: String,
    pub environment: TargetEnvironment,
    pub byte_len: usize,
}

pub struct SecretVault<S: SecretStore> {
    store: S,
}

impl<S: SecretStore> SecretVault<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn store_profile_payload(
        &self,
        profile_id: &str,
        environment: TargetEnvironment,
        payload: &str,
    ) -> Result<SecretEnvelope, SecretStoreError> {
        let key = secret_key(profile_id, environment);
        self.store.put_secret(&key, payload)?;
        Ok(SecretEnvelope {
            key,
            environment,
            byte_len: payload.len(),
        })
    }

    pub fn load_profile_payload(
        &self,
        profile_id: &str,
        environment: TargetEnvironment,
    ) -> Result<Option<String>, SecretStoreError> {
        self.store.get_secret(&secret_key(profile_id, environment))
    }

    pub fn delete_profile_payload(
        &self,
        profile_id: &str,
        environment: TargetEnvironment,
    ) -> Result<(), SecretStoreError> {
        self.store.delete_secret(&secret_key(profile_id, environment))
    }
}

pub fn secret_key(profile_id: &str, environment: TargetEnvironment) -> String {
    format!("profile:{profile_id}:environment:{}", environment.key())
}

#[derive(Clone, Default)]
pub struct MemorySecretStore {
    values: Arc<Mutex<HashMap<String, String>>>,
}

impl SecretStore for MemorySecretStore {
    fn put_secret(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        let mut values = self.values.lock().map_err(|_| SecretStoreError::LockPoisoned)?;
        values.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get_secret(&self, key: &str) -> Result<Option<String>, SecretStoreError> {
        let values = self.values.lock().map_err(|_| SecretStoreError::LockPoisoned)?;
        Ok(values.get(key).cloned())
    }

    fn delete_secret(&self, key: &str) -> Result<(), SecretStoreError> {
        let mut values = self.values.lock().map_err(|_| SecretStoreError::LockPoisoned)?;
        values.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_stable_secret_keys() {
        assert_eq!(
            secret_key("profile-1", TargetEnvironment::Desktop),
            "profile:profile-1:environment:desktop"
        );
    }

    #[test]
    fn memory_vault_stores_loads_and_deletes_payload() {
        let vault = SecretVault::new(MemorySecretStore::default());

        let envelope = vault
            .store_profile_payload("profile-1", TargetEnvironment::Cli, "{\"mock\":true}")
            .expect("store payload");

        assert_eq!(envelope.environment, TargetEnvironment::Cli);
        assert_eq!(envelope.byte_len, 13);
        assert_eq!(
            vault
                .load_profile_payload("profile-1", TargetEnvironment::Cli)
                .expect("load payload"),
            Some("{\"mock\":true}".to_string())
        );

        vault
            .delete_profile_payload("profile-1", TargetEnvironment::Cli)
            .expect("delete payload");
        assert_eq!(
            vault
                .load_profile_payload("profile-1", TargetEnvironment::Cli)
                .expect("load missing payload"),
            None
        );
    }
}

