/// Represents an identity key pair (public + private)
/// Agnostic structure usable in both WASM and Native
#[derive(Clone, Debug)]
pub struct IdentityKeys {
    pub public_key: String,
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
