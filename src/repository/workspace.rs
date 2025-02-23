use std::{
    fs, io,
    path::{Path, PathBuf},
};

use lexical_sort::natural_lexical_cmp;
use walkdir::WalkDir;

pub struct Workspace {
    pub root: PathBuf,
}

impl Workspace {
    pub fn new(path: PathBuf) -> Self {
        Self { root: path }
    }

    pub fn list_files(&self) -> Result<Vec<PathBuf>, anyhow::Error> {
        let db_path = self.root.join(".git");

        let mut list_result: Vec<PathBuf> = Vec::new();
        for entry in WalkDir::new(&self.root)
            .follow_links(false)
            .follow_root_links(false)
            .same_file_system(true)
            .into_iter()
            .filter_entry(|e| !e.path().starts_with(&db_path))
        {
            let entry = entry?;

            if !entry.metadata()?.is_dir() {
                let relative_path = entry.path().strip_prefix(&self.root)?;
                list_result.push(relative_path.to_owned());
            }
        }

        list_result.sort_by(|a, b| natural_lexical_cmp(a.to_str().unwrap(), b.to_str().unwrap()));
        Ok(list_result)
    }

    pub fn read_file(&self, path: &Path) -> Result<Vec<u8>, io::Error> {
        fs::read(self.root.join(path))
    }
}
