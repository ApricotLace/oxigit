use crate::oid::Oid;
use anyhow::Context;
use std::{fs, fs::File, io::Write, path::PathBuf};

pub struct Refs {
    root: PathBuf,
}

impl Refs {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn set_head(&self, oid: &Oid) -> Result<(), anyhow::Error> {
        File::create(self.root.join("HEAD"))?
            .write_all(oid.to_string().as_bytes())
            .with_context(|| "Could not write HEAD reference")
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
