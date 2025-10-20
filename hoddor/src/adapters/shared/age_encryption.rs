use crate::ports::EncryptionPort;
use age::{
    x25519::{Identity, Recipient},
    Decryptor, Encryptor,
};
use async_trait::async_trait;
use futures::io::{AllowStdIo, AsyncReadExt, AsyncWriteExt};
use std::error::Error;
use std::io::Cursor;

#[derive(Clone, Copy, Debug)]
pub struct AgeEncryption;

impl AgeEncryption {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgeEncryption {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl EncryptionPort for AgeEncryption {
    async fn encrypt(&self, data: &[u8], recipients: &[&str]) -> Result<Vec<u8>, Box<dyn Error>> {
        let parsed_recipients: Result<Vec<Recipient>, _> =
            recipients.iter().map(|r| r.parse()).collect();
        let parsed = parsed_recipients?;

        if parsed.is_empty() {
            return Err("No recipients provided".into());
        }

        let encryptor = Encryptor::with_recipients(
            parsed
                .iter()
                .map(|r| Box::new(r.clone()) as Box<dyn age::Recipient + Send>)
                .collect(),
        )
        .ok_or("Failed to create encryptor")?;

        let mut encrypted = vec![];
        let cursor = Cursor::new(&mut encrypted);
        let async_cursor = AllowStdIo::new(cursor);
        let mut writer = encryptor.wrap_output(Box::new(async_cursor))?;

        AsyncWriteExt::write_all(&mut writer, data).await?;
        AsyncWriteExt::close(&mut writer).await?;

        Ok(encrypted)
    }

    async fn decrypt(
        &self,
        encrypted: &[u8],
        identity_str: &str,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let identity: Identity = identity_str.parse()?;

        let decryptor = Decryptor::new(encrypted)?;

        match decryptor {
            Decryptor::Recipients(d) => {
                let mut decrypted = vec![];
                let reader = d.decrypt(std::iter::once(&identity as &dyn age::Identity))?;
                let mut async_reader = AllowStdIo::new(reader);
                AsyncReadExt::read_to_end(&mut async_reader, &mut decrypted).await?;
                Ok(decrypted)
            }
            _ => Err("File was not encrypted with recipients".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use age::secrecy::ExposeSecret;
    use futures::executor::block_on;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let adapter = AgeEncryption::new();
        let identity = age::x25519::Identity::generate();
        let recipient = identity.to_public().to_string();

        let data = b"secret message";
        let encrypted = block_on(adapter.encrypt(data, &[&recipient])).unwrap();

        let identity_str = identity.to_string().expose_secret().to_string();
        let decrypted = block_on(adapter.decrypt(&encrypted, &identity_str)).unwrap();

        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encrypt_no_recipients() {
        let adapter = AgeEncryption::new();
        let data = b"secret";
        let result = block_on(adapter.encrypt(data, &[]));
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_multiple_recipients() {
        let adapter = AgeEncryption::new();
        let identity1 = age::x25519::Identity::generate();
        let identity2 = age::x25519::Identity::generate();
        let recipient1 = identity1.to_public().to_string();
        let recipient2 = identity2.to_public().to_string();

        let data = b"multi-recipient message";
        let encrypted = block_on(adapter.encrypt(data, &[&recipient1, &recipient2])).unwrap();

        let identity1_str = identity1.to_string().expose_secret().to_string();
        let decrypted1 = block_on(adapter.decrypt(&encrypted, &identity1_str)).unwrap();
        assert_eq!(decrypted1, data);

        let identity2_str = identity2.to_string().expose_secret().to_string();
        let decrypted2 = block_on(adapter.decrypt(&encrypted, &identity2_str)).unwrap();
        assert_eq!(decrypted2, data);
    }
}
