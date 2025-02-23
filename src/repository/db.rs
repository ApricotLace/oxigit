use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

use anyhow::Context;

use flate2::{write::ZlibEncoder, Compression};
use rand::distributions::{Alphanumeric, DistString};

use crate::oid::Oid;

use super::object::Object;

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

    pub fn store_object(&self, object: &mut impl Object) -> Result<Oid, anyhow::Error> {
        let serialized_object = object.to_bytes();
        let mut content: Vec<u8> = vec![];
        content.extend_from_slice(object.kind());
        content.push(b' ');
        content.extend_from_slice(serialized_object.len().to_string().as_bytes());
        content.push(0);
        content.extend_from_slice(&serialized_object);

        let oid = Oid::new(&content);
        object.set_oid(oid.clone());

        self.write_object(&oid.to_string(), &content)?;

        Ok(oid)
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
