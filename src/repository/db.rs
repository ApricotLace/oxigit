use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

use anyhow::Context;

use flate2::{write::ZlibEncoder, Compression};
use rand::distributions::{Alphanumeric, DistString};

use crate::oid::Oid;

use super::object::{DbObject, Object};

pub struct Db {
    root: PathBuf,
}

impl Db {
    fn objects_path(&self) -> PathBuf {
        self.root.join("objects")
    }

    pub fn new(db_path: PathBuf) -> Self {
        Self { root: db_path }
    }

    pub fn init(&self) -> Result<(), io::Error> {
        fs::create_dir_all(self.objects_path())?;
        fs::create_dir_all(self.root.join("refs"))?;
        Ok(())
    }

    pub fn store_object(&self, object: Object) -> Result<Oid, anyhow::Error> {
        let object: DbObject = object.into();

        self.write_object(&object.oid().to_string(), object.data())?;

        Ok(*object.oid())
    }

    pub fn write_object(&self, oid: &str, content: &[u8]) -> Result<(), anyhow::Error> {
        let (group, rest) = oid.split_at(2);
        let group_path = self.objects_path().join(group);
        let object_path = group_path.join(rest);

        if let Ok(true) = fs::exists(&object_path) {
            return Ok(());
        }

        let temp_path = group_path.join(generate_temp_name());

        fs::create_dir_all(group_path)?;

        let file = File::create_new(&temp_path).with_context(|| "Failed to open temp file.")?;

        let mut encoder = ZlibEncoder::new(file, Compression::default());

        encoder
            .write_all(content)
            .with_context(|| "Failed to write compressed content.")?;

        fs::rename(&temp_path, &object_path).with_context(|| "Failed to rename file")?;

        Ok(())
    }
}

fn generate_temp_name() -> String {
    let suffix = Alphanumeric.sample_string(&mut rand::thread_rng(), 6);
    format!("tmp_obj_{suffix}")
}
