use std::{
    collections::{btree_map::Entry, BTreeMap},
    ffi::{CStr, CString},
};

use anyhow::anyhow;

use crate::oid::Oid;

const MODE: usize = 100644;

#[derive(Debug, Clone)]
pub struct Tree {
    entries: BTreeMap<CString, Oid>,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    fn serialize(name: &CStr, oid: &Oid) -> Vec<u8> {
        let mut serialized = Vec::new();
        serialized.extend_from_slice(format!("{MODE}").as_bytes());
        serialized.push(b' ');
        serialized.extend_from_slice(name.to_bytes_with_nul());
        serialized.extend_from_slice(oid.as_bytes());
        serialized
    }

    pub fn add_entry(&mut self, path: CString, oid: Oid) -> Result<(), anyhow::Error> {
        match self.entries.entry(path) {
            Entry::Vacant(e) => e.insert(oid),
            Entry::Occupied(e) => {
                return Err(anyhow!(
                    "Duplicate tree entry: {}",
                    e.key().to_string_lossy()
                ))
            }
        };

        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.entries
            .iter()
            .flat_map(|(name, oid)| Tree::serialize(name, oid))
            .collect()
    }
}
