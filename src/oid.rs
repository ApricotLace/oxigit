use hex;
use sha1::{Digest, Sha1};
use std::fmt::{Debug, Display};

#[derive(Clone, Copy)]
pub struct Oid {
    hash: [u8; 20],
}

impl Oid {
    pub fn new(data: &[u8]) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(data);
        let hash = hasher.finalize();
        Self { hash: hash.into() }
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.hash
    }
}

impl Debug for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", base16ct::lower::encode_string(&self.hash))
    }
}

impl Display for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", base16ct::lower::encode_string(&self.hash))
    }
}

impl From<Oid> for String {
    fn from(value: Oid) -> Self {
        value.to_string()
    }
}

impl From<String> for Oid {
    fn from(value: String) -> Self {
        Self {
            hash: hex::decode(value).unwrap().try_into().unwrap(),
        }
    }
}
