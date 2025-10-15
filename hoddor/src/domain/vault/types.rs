use std::collections::HashMap;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Expiration {
    pub expires_at: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct NamespaceData {
    pub data: Vec<u8>,
    pub expiration: Option<Expiration>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct VaultMetadata {
    pub peer_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct IdentitySalts {
    salts: HashMap<String, [u8; 32]>,
    credential_ids: HashMap<String, Vec<u8>>,
}

impl Default for IdentitySalts {
    fn default() -> Self {
        Self::new()
    }
}

impl IdentitySalts {
    pub fn new() -> Self {
        Self {
            salts: HashMap::new(),
            credential_ids: HashMap::new(),
        }
    }

    pub fn get_salt(&self, public_key: &str) -> Option<&[u8; 32]> {
        self.salts.get(public_key)
    }

    pub fn set_salt(&mut self, public_key: String, salt: [u8; 32]) {
        self.salts.insert(public_key, salt);
    }

    pub fn get_all_salts(&self) -> impl Iterator<Item = &[u8; 32]> {
        self.salts.values()
    }

    pub fn salts_iter(&self) -> impl Iterator<Item = (&String, &[u8; 32])> {
        self.salts.iter()
    }

    pub fn get_all_credential_ids(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.credential_ids.values()
    }

    pub fn get_credential_id(&self, public_key: &str) -> Option<&Vec<u8>> {
        self.credential_ids.get(public_key)
    }

    pub fn set_credential_id(&mut self, public_key: String, credential_id: Vec<u8>) {
        self.credential_ids.insert(public_key, credential_id);
    }

    pub fn get_public_keys_with_credentials(&self) -> impl Iterator<Item = &String> {
        self.credential_ids.keys()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Vault {
    pub metadata: VaultMetadata,
    pub identity_salts: IdentitySalts,
    pub username_pk: HashMap<String, String>,
    pub namespaces: HashMap<String, NamespaceData>,
    pub sync_enabled: bool,
}
