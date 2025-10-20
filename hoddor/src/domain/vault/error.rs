use std::fmt;

#[derive(Debug, Clone)]
pub enum VaultError {
    IoError(String),
    NamespaceNotFound,
    InvalidPassword,
    SerializationError(String),
    DataExpired,
    NamespaceAlreadyExists,
    VaultAlreadyExists,
    VaultNotFound,
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaultError::IoError(msg) => write!(f, "IO Error: {msg}"),
            VaultError::NamespaceNotFound => write!(f, "Namespace not found"),
            VaultError::InvalidPassword => write!(f, "Invalid password"),
            VaultError::SerializationError(msg) => write!(f, "Serialization Error: {msg}"),
            VaultError::DataExpired => write!(f, "Data has expired"),
            VaultError::NamespaceAlreadyExists => write!(f, "Namespace already exists"),
            VaultError::VaultAlreadyExists => write!(f, "Vault already exists"),
            VaultError::VaultNotFound => write!(f, "Vault not found"),
        }
    }
}

impl std::error::Error for VaultError {}

impl VaultError {
    pub fn io_error(message: impl Into<String>) -> Self {
        VaultError::IoError(message.into())
    }

    pub fn serialization_error(message: impl Into<String>) -> Self {
        VaultError::SerializationError(message.into())
    }
}
