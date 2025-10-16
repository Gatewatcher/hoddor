/// Represents an identity key pair (public + private)
/// Agnostic structure usable in both WASM and Native
#[derive(Clone, Debug)]
pub struct IdentityKeys {
    /// Public key in Age format (age1...)
    pub public_key: String,
    /// Private key in Age format (AGE-SECRET-KEY-1...)
    pub private_key: String,
}

impl IdentityKeys {
    pub fn new(public_key: String, private_key: String) -> Self {
        Self {
            public_key,
            private_key,
        }
    }
}
