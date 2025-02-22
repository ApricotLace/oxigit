use crate::lockfile::Lockfile;
use crate::oid::Oid;
use anyhow::Context;
use std::{fs, path::PathBuf};

pub struct Refs {
    root: PathBuf,
}

impl Refs {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn set_head(&self, oid: &Oid) -> Result<(), anyhow::Error> {
        let head_path = self.root.join("HEAD");
        let mut lockfile = Lockfile::new(head_path.clone());

        lockfile
            .hold_for_update()
            .with_context(|| format!("Could not acquire lock on file: {:?}", head_path.clone()))?;

        lockfile
            .write((oid.to_string() + "\n").as_bytes())
            .with_context(|| "Could not write HEAD reference")?;
        lockfile.commit()?;
        Ok(())
    }

    fn head_path(&self) -> PathBuf {
        self.root.join("HEAD")
    }

    pub fn get_head(&self) -> Result<Oid, anyhow::Error> {
        let content = fs::read_to_string(&self.head_path())
            .with_context(|| "Could not read HEAD reference")?;
        Ok(Oid::from(content))
    }
}
