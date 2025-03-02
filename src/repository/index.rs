use lexical_sort::natural_lexical_cmp;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs::{File, Metadata};
use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt;
use std::{ffi::CString, path::PathBuf};

use crate::checksum::Checksum;
use crate::lockfile::Lockfile;
use crate::oid::Oid;
use anyhow;

const ENTRY_BLOCK: usize = 8;
const ENTRY_MIN_SIZE: usize = 64;
const EXECUTABLE_MODE: u32 = 0o100755;
const HEADER_SIZE: usize = 12;
const MAX_PATH_SIZE: usize = 0xFFF;
const REGULAR_MODE: u32 = 0o100644;
const SIGNATURE: &[u8] = b"DIRC";
const VERSION: u32 = 2;

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
    pub oid: Oid,
    flags: u16,
    pub path: CString,
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
        let path = CString::new(pathname.to_str().unwrap())?;
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

    fn parse(data: &[u8]) -> Result<Self, anyhow::Error> {
        if data.len() < ENTRY_MIN_SIZE {
            return Err(anyhow::anyhow!("Entry data too short"));
        }

        let ctime = i32::from_be_bytes(data[0..4].try_into()?);
        let ctime_nsec = u32::from_be_bytes(data[4..8].try_into()?);
        let mtime = i32::from_be_bytes(data[8..12].try_into()?);
        let mtime_nsec = u32::from_be_bytes(data[12..16].try_into()?);
        let dev = u32::from_be_bytes(data[16..20].try_into()?);
        let ino = u32::from_be_bytes(data[20..24].try_into()?);
        let mode = u32::from_be_bytes(data[24..28].try_into()?);
        let uid = u32::from_be_bytes(data[28..32].try_into()?);
        let gid = u32::from_be_bytes(data[32..36].try_into()?);
        let size = u32::from_be_bytes(data[36..40].try_into()?);

        let mut oid_bytes = [0u8; 20];
        oid_bytes.copy_from_slice(&data[40..60]);
        let oid = Oid::from(&oid_bytes[..]);

        let flags = u16::from_be_bytes(data[60..62].try_into()?);

        let path_end = data[62..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| anyhow::anyhow!("Path not null-terminated"))?;

        let path = CString::new(&data[62..62 + path_end])?;

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
}

#[derive(Eq, PartialEq)]
pub struct NaturalCString(CString);

impl PartialOrd for NaturalCString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NaturalCString {
    fn cmp(&self, other: &Self) -> Ordering {
        natural_lexical_cmp(
            self.0.to_str().unwrap_or_default(),
            other.0.to_str().unwrap_or_default(),
        )
    }
}

pub struct Index {
    lockfile: Lockfile,
    changed: bool,
    pub entries: BTreeMap<NaturalCString, IndexEntry>,
}

impl Index {
    pub fn new(root_path: PathBuf) -> Self {
        Index {
            lockfile: Lockfile::new(root_path.join("index")),
            changed: false,
            entries: BTreeMap::new(),
        }
    }
    fn open_index_file(&self) -> Result<Option<File>, anyhow::Error> {
        match File::open(&self.lockfile.file_path) {
            Ok(file) => Ok(Some(file)),
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(err.into())
                }
            }
        }
    }

    fn read_header(&self, reader: &mut Checksum<File>) -> Result<u32, anyhow::Error> {
        let data = reader.read(HEADER_SIZE)?;

        let signature = &data[0..4];
        if signature != SIGNATURE {
            return Err(anyhow::anyhow!(
                "Signature: expected 'DIRC' but found '{}'",
                String::from_utf8_lossy(signature)
            ));
        }

        let version = u32::from_be_bytes(data[4..8].try_into()?);
        if version != VERSION {
            return Err(anyhow::anyhow!(
                "Version: expected '{}' but found '{}'",
                VERSION,
                version
            ));
        }

        let count = u32::from_be_bytes(data[8..12].try_into()?);

        Ok(count)
    }

    fn read_entry(&self, reader: &mut Checksum<File>) -> Result<IndexEntry, anyhow::Error> {
        let mut entry_data = reader.read(ENTRY_MIN_SIZE)?;

        while entry_data.last() != Some(&0) {
            entry_data.extend(reader.read(ENTRY_BLOCK)?);
        }

        IndexEntry::parse(&entry_data)
    }

    fn read_entries(
        &mut self,
        reader: &mut Checksum<File>,
        count: u32,
    ) -> Result<(), anyhow::Error> {
        for _ in 0..count {
            let entry = self.read_entry(reader)?;
            self.entries
                .insert(NaturalCString(entry.path.clone()), entry);
        }
        Ok(())
    }

    pub fn load(&mut self) -> Result<(), anyhow::Error> {
        let file = self.open_index_file()?;

        if let Some(file) = file {
            let mut rdr = Checksum::new(file);
            let count = self.read_header(&mut rdr)?;
            self.read_entries(&mut rdr, count)?;
            rdr.verify_checksum()?;
            return Ok(());
        }

        Ok(())
    }

    pub fn load_for_update(&mut self) -> Result<bool, anyhow::Error> {
        if !self.lockfile.hold_for_update()? {
            return Ok(false);
        }
        self.load()?;
        Ok(true)
    }

    pub fn add(
        &mut self,
        path: PathBuf,
        oid: Oid,
        stat: std::fs::Metadata,
    ) -> Result<(), anyhow::Error> {
        self.entries.insert(
            NaturalCString(CString::new(path.to_str().unwrap())?),
            IndexEntry::new(path, oid, stat)?,
        );
        self.changed = true;
        Ok(())
    }

    pub fn write_updates(&mut self) -> Result<bool, anyhow::Error> {
        if !self.changed {
            self.lockfile.rollback()?;
        }

        let mut writer = Checksum::new(self.lockfile.lock.as_ref().unwrap());

        let entries_len = self.entries.len() as u32;
        let dirc = b"DIRC";
        let version: u32 = 2;

        let header = [
            &dirc[..],
            &version.to_be_bytes(),
            &entries_len.to_be_bytes(),
        ]
        .concat();

        writer.write(&header)?;

        for entry in self.entries.values() {
            writer.write(&entry.to_bytes())?;
        }

        writer.write_checksum()?;
        self.lockfile.commit()?;
        self.changed = false;

        Ok(true)
    }
}
