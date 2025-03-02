use anyhow;
use sha1::{Digest, Sha1};
use std::io::{self, Read, Write};

const CHECKSUM_SIZE: usize = 20;

pub struct Checksum<R: Read> {
    file: R,
    digest: Sha1,
}

impl<R: Read + Write> Checksum<R> {
    pub fn new(file: R) -> Self {
        Self {
            file,
            digest: Sha1::new(),
        }
    }

    pub fn read(&mut self, size: usize) -> Result<Vec<u8>, anyhow::Error> {
        let mut data = vec![0; size];
        match self.file.read_exact(&mut data) {
            Ok(_) => {
                self.digest.update(&data);
                Ok(data)
            }
            Err(e) => match e.kind() {
                io::ErrorKind::UnexpectedEof => Err(anyhow::anyhow!("Unexpected end-of-file",)),
                _ => Err(e.into()),
            },
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), anyhow::Error> {
        self.file.write_all(data)?;
        self.digest.update(data);
        Ok(())
    }

    pub fn write_checksum(&mut self) -> Result<(), anyhow::Error> {
        self.file
            .write_all(self.digest.clone().finalize().as_slice())?;
        Ok(())
    }

    pub fn verify_checksum(&mut self) -> Result<(), anyhow::Error> {
        let mut sum = vec![0; CHECKSUM_SIZE];
        self.file.read_exact(&mut sum)?;

        let computed = self.digest.clone().finalize();
        if sum != computed.as_slice() {
            return Err(anyhow::anyhow!(
                "Checksum does not match value stored on disk"
            ));
        }

        Ok(())
    }
}
