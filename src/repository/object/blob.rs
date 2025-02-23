use super::Object;
use crate::oid::Oid;

#[derive(Debug, Clone)]
pub struct Blob {
    oid: Option<Oid>,
    data: Vec<u8>,
}

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Self { oid: None, data }
    }
}

impl Object for Blob {
    fn kind(&self) -> &[u8] {
        b"blob"
    }

    fn set_oid(&mut self, oid: Oid) {
        self.oid = Some(oid);
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.data.to_owned()
    }
}
