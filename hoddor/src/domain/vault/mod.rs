pub mod expiration;
pub mod operations;
pub mod serialization;
pub mod types;
pub mod validation;

pub use expiration::{cleanup_expired_namespaces, create_expiration, is_expired};
pub use operations::{
    create_vault, create_vault_from_sync, delete_namespace_file, delete_vault,
    get_namespace_filename, list_vaults, read_vault, save_vault,
};
pub use serialization::{deserialize_vault, serialize_vault};
pub use types::{Expiration, IdentitySalts, NamespaceData, Vault, VaultMetadata};
pub use validation::{validate_namespace, validate_passphrase, validate_vault_name};
