use indexmap::IndexMap;
use sha1::{Digest, Sha1};
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::{ffi::CString, path::PathBuf};

use crate::lockfile::Lockfile;
use crate::oid::Oid;
use anyhow::{self, Context};

const REGULAR_MODE: u32 = 0o100644;
const EXECUTABLE_MODE: u32 = 0o100755;
const MAX_PATH_SIZE: usize = 0xFFF;

#[derive(Debug)]
pub struct IndexEntry {
    ctime: i32,
    ctime_nsec: u32,
    mtime: i32,
    mtime_nsec: u32,
    dev: u32,
    ino: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    size: u32,
    oid: Oid,
    flags: u16,
    path: CString,
}

impl IndexEntry {
    pub fn new(pathname: PathBuf, oid: Oid, metadata: Metadata) -> Result<Self, anyhow::Error> {
        let ctime = metadata.ctime() as i32;
        let ctime_nsec = metadata.ctime_nsec() as u32;
        let mtime = metadata.mtime() as i32;
        let mtime_nsec = metadata.mtime_nsec() as u32;
        let dev = metadata.dev() as u32;
        let ino = metadata.ino() as u32;
        let mode = if metadata.mode() & 0o111 != 0 {
            EXECUTABLE_MODE
        } else {
            REGULAR_MODE
        };
        let uid = metadata.uid();
        let gid = metadata.gid();
        let size = metadata.size() as u32;
        let path = CString::new(
            pathname
                .to_str()
                .with_context(|| "Can't convert path to CString")?,
        )?;
        let flags = std::cmp::min(path.as_bytes().len(), MAX_PATH_SIZE) as u16;

        Ok(IndexEntry {
            ctime,
            ctime_nsec,
            mtime,
            mtime_nsec,
            dev,
            ino,
            mode,
            uid,
            gid,
            size,
            oid,
            flags,
            path,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.ctime.to_be_bytes());
        bytes.extend_from_slice(&self.ctime_nsec.to_be_bytes());
        bytes.extend_from_slice(&self.mtime.to_be_bytes());
        bytes.extend_from_slice(&self.mtime_nsec.to_be_bytes());
        bytes.extend_from_slice(&self.dev.to_be_bytes());
        bytes.extend_from_slice(&self.ino.to_be_bytes());
        bytes.extend_from_slice(&self.mode.to_be_bytes());
        bytes.extend_from_slice(&self.uid.to_be_bytes());
        bytes.extend_from_slice(&self.gid.to_be_bytes());
        bytes.extend_from_slice(&self.size.to_be_bytes());

        bytes.extend_from_slice(self.oid.as_bytes());

        bytes.extend_from_slice(&self.flags.to_be_bytes());

        bytes.extend_from_slice(self.path.as_bytes());
        bytes.push(0);

        while bytes.len() % 8 != 0 {
            bytes.push(0);
        }

        bytes
    }
}

pub struct Index {
    lockfile: Lockfile,
    entries: IndexMap<CString, IndexEntry>,
}

impl Index {
    pub fn new(root_path: PathBuf) -> Self {
        Index {
            lockfile: Lockfile::new(root_path.join("index")),
            entries: IndexMap::new(),
        }
    }

    pub fn add(
        &mut self,
        path: PathBuf,
        oid: Oid,
        stat: std::fs::Metadata,
    ) -> Result<(), anyhow::Error> {
        self.entries.insert(
            CString::new(
                path.to_str()
                    .with_context(|| "Can't convert path to CString")?,
            )?,
            IndexEntry::new(path, oid, stat)?,
        );
        Ok(())
    }

    fn write(&self, data: &[u8], hasher: &mut Sha1) -> Result<(), anyhow::Error> {
        self.lockfile.write(data)?;
        hasher.update(data);
        Ok(())
    }

    fn finish_write(&mut self, hasher: Sha1) -> Result<(), anyhow::Error> {
        let h = hasher.finalize();
        self.lockfile.write(&h)?;
        self.lockfile.commit()?;
        Ok(())
    }

    pub fn write_updates(&mut self) -> Result<bool, anyhow::Error> {
        if !self.lockfile.hold_for_update()? {
            return Ok(false);
        }

        let mut hasher = Sha1::new();

        let entries_len = self.entries.len() as u32;
        let dirc = b"DIRC";
        let version: u32 = 2;

        let header = [
            &dirc[..],
            &version.to_be_bytes(),
            &entries_len.to_be_bytes(),
        ]
        .concat();

        self.write(&header, &mut hasher)?;

        for entry in self.entries.values() {
            self.write(&entry.to_bytes(), &mut hasher)?;
        }

        self.finish_write(hasher)?;
        Ok(true)
    }
}
