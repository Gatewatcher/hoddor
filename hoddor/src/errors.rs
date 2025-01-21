use wasm_bindgen::JsValue;
use std::fmt;

#[derive(Debug)]
pub enum VaultError {
    IoError { message: &'static str },
    NamespaceNotFound,
    InvalidPassword,
    SerializationError { message: &'static str },
    JsError(String),
    DataExpired,
    NamespaceAlreadyExists,
    VaultAlreadyExists,
    VaultNotFound,
}

impl From<JsValue> for VaultError {
    fn from(err: JsValue) -> Self {
        VaultError::JsError(
            err.as_string()
                .unwrap_or_else(|| "Unknown JS error".to_string()),
        )
    }
}

impl From<LockError> for VaultError {
    fn from(error: LockError) -> Self {
        match error {
            LockError::AcquisitionFailed => VaultError::IoError {
                message: "Failed to acquire lock",
            },
        }
    }
}

#[derive(Debug)]
pub enum LockError {
    AcquisitionFailed,
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaultError::IoError { message } => write!(f, "IO Error: {}", message),
            VaultError::NamespaceNotFound => write!(f, "Namespace not found"),
            VaultError::InvalidPassword => write!(f, "Invalid password"),
            VaultError::SerializationError { message } => write!(f, "Serialization Error: {}", message),
            VaultError::JsError(msg) => write!(f, "JavaScript Error: {}", msg),
            VaultError::DataExpired => write!(f, "Data has expired"),
            VaultError::NamespaceAlreadyExists => write!(f, "Namespace already exists"),
            VaultError::VaultAlreadyExists => write!(f, "Vault already exists"),
            VaultError::VaultNotFound => write!(f, "Vault not found"),
        }
    }
}

impl From<VaultError> for JsValue {
    fn from(error: VaultError) -> Self {
        match error {
            VaultError::IoError { message } => JsValue::from_str(&format!("IO Error: {}", message)),
            VaultError::NamespaceNotFound => JsValue::from_str("Namespace not found"),
            VaultError::InvalidPassword => JsValue::from_str("Invalid password"),
            VaultError::SerializationError { message } => {
                JsValue::from_str(&format!("Serialization Error: {}", message))
            }
            VaultError::JsError(msg) => JsValue::from_str(&format!("JavaScript Error: {}", msg)),
            VaultError::DataExpired => JsValue::from_str("Data has expired"),
            VaultError::NamespaceAlreadyExists => JsValue::from_str("Namespace already exists"),
            VaultError::VaultAlreadyExists => JsValue::from_str("Vault already exists"),
            VaultError::VaultNotFound => JsValue::from_str("Vault not found"),
        }
    }
}
