use keyring::{Entry, Error};

const SERVICE_NAME: &str = "io.github.sahel.Echo";
const ACCOUNT_NAME: &str = "groq-api-key";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiKeyStatus {
    Saved,
    Missing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecretError {
    EmptyKey,
    StorageUnavailable,
}

pub struct ApiKeyStore {
    entry: Entry,
}

impl ApiKeyStore {
    pub fn open() -> Result<Self, SecretError> {
        Entry::new(SERVICE_NAME, ACCOUNT_NAME)
            .map(|entry| Self { entry })
            .map_err(|_| SecretError::StorageUnavailable)
    }

    pub fn status(&self) -> Result<ApiKeyStatus, SecretError> {
        match self.entry.get_password() {
            Ok(_) => Ok(ApiKeyStatus::Saved),
            Err(Error::NoEntry) => Ok(ApiKeyStatus::Missing),
            Err(_) => Err(SecretError::StorageUnavailable),
        }
    }

    pub fn save(&self, key: &str) -> Result<(), SecretError> {
        if key.trim().is_empty() {
            return Err(SecretError::EmptyKey);
        }

        self.entry
            .set_password(key)
            .map_err(|_| SecretError::StorageUnavailable)
    }

    pub fn remove(&self) -> Result<(), SecretError> {
        match self.entry.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(_) => Err(SecretError::StorageUnavailable),
        }
    }

    #[cfg(test)]
    fn from_entry(entry: Entry) -> Self {
        Self { entry }
    }
}

pub fn api_key_status() -> Result<ApiKeyStatus, SecretError> {
    ApiKeyStore::open()?.status()
}

pub fn save_api_key(key: &str) -> Result<(), SecretError> {
    ApiKeyStore::open()?.save(key)
}

pub fn remove_api_key() -> Result<(), SecretError> {
    ApiKeyStore::open()?.remove()
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyring::mock::MockCredential;

    fn mock_store() -> ApiKeyStore {
        ApiKeyStore::from_entry(Entry::new_with_credential(Box::new(
            MockCredential::default(),
        )))
    }

    #[test]
    fn saves_replaces_detects_and_removes_a_key_without_exposing_it_in_status() {
        let store = mock_store();

        assert_eq!(store.status().expect("status reads"), ApiKeyStatus::Missing);
        store.save("test-key-one").expect("first key saves");
        assert_eq!(store.status().expect("status reads"), ApiKeyStatus::Saved);

        store.save("test-key-two").expect("replacement saves");
        assert_eq!(store.status().expect("status reads"), ApiKeyStatus::Saved);
        assert_eq!(
            store.entry.get_password().expect("replacement is stored"),
            "test-key-two"
        );

        store.remove().expect("key removes");
        assert_eq!(store.status().expect("status reads"), ApiKeyStatus::Missing);
    }

    #[test]
    fn empty_key_is_rejected_before_reaching_secure_storage() {
        assert_eq!(mock_store().save("  "), Err(SecretError::EmptyKey));
    }

    #[test]
    fn storage_failures_do_not_surface_keyring_details() {
        let entry = Entry::new_with_credential(Box::new(MockCredential::default()));
        let credential = entry
            .get_credential()
            .downcast_ref::<MockCredential>()
            .expect("mock credential is available");
        credential.set_error(Error::NoStorageAccess(Box::new(std::io::Error::other(
            "test storage failure",
        ))));
        let store = ApiKeyStore::from_entry(entry);

        assert_eq!(store.status(), Err(SecretError::StorageUnavailable));
    }

    #[test]
    #[ignore = "requires a running desktop Secret Service"]
    fn real_secret_service_saves_survives_a_fresh_store_replaces_and_removes() {
        let store = test_secret_service_store();
        store.remove().expect("removing a prior test key succeeds");
        store.save("temporary-test-key").expect("test key saves");
        assert_eq!(
            test_secret_service_store()
                .status()
                .expect("fresh store reads status"),
            ApiKeyStatus::Saved
        );

        store
            .save("replacement-temporary-test-key")
            .expect("test key replaces");
        assert_eq!(
            test_secret_service_store()
                .status()
                .expect("replacement remains saved"),
            ApiKeyStatus::Saved
        );

        store.remove().expect("test key removes");
        assert_eq!(
            test_secret_service_store()
                .status()
                .expect("removed key is absent"),
            ApiKeyStatus::Missing
        );
    }

    fn test_secret_service_store() -> ApiKeyStore {
        ApiKeyStore::from_entry(
            Entry::new(SERVICE_NAME, "key-001-test-groq-api-key")
                .expect("test Secret Service entry is available"),
        )
    }
}
