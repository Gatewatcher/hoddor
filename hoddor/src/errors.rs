use wasm_bindgen::JsValue;

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
