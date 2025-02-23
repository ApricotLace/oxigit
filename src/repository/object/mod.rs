use crate::oid::Oid;

pub mod blob;
pub mod commit;
pub mod tree;

pub trait Object {
    fn kind(&self) -> &[u8];
    fn set_oid(&mut self, oid: Oid);
    fn to_bytes(&self) -> Vec<u8>;
}
