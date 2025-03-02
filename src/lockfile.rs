use anyhow::{self, Context};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub struct Lockfile {
    pub file_path: PathBuf,
    lock_path: PathBuf,
    pub lock: Option<File>,
}

impl Lockfile {
    pub fn new(file_path: PathBuf) -> Self {
        Lockfile {
            file_path: file_path.clone(),
            lock_path: file_path.clone().with_extension("lock"),
            lock: None,
        }
    }

    pub fn hold_for_update(&mut self) -> Result<bool, anyhow::Error> {
        match self.lock {
            Some(_) => Ok(true),
            None => match OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&self.lock_path)
            {
                Ok(lock) => {
                    self.lock = Some(lock);
                    Ok(true)
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::AlreadyExists => Ok(false),
                    _ => Err(e.into()),
                },
            },
        }
    }

    pub fn write(&self, content: &[u8]) -> Result<(), anyhow::Error> {
        self.fail_on_stale_lock()?;
        self.lock
            .as_ref()
            .with_context(|| "Lock dissappeared?!")?
            .write_all(content)?;
        Ok(())
    }

    pub fn commit(&mut self) -> Result<(), anyhow::Error> {
        self.fail_on_stale_lock()?;
        self.lock = None;
        fs::rename(&self.lock_path, &self.file_path)?;

        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), anyhow::Error> {
        self.fail_on_stale_lock()?;
        self.lock = None;
        fs::remove_file(&self.lock_path)?;
        Ok(())
    }

    fn fail_on_stale_lock(&self) -> Result<(), anyhow::Error> {
        match self.lock {
            None => Err(anyhow::anyhow!(
                "Not holding lock on file: {:?}",
                self.lock_path
            )),
            Some(_) => Ok(()),
        }
    }
}
