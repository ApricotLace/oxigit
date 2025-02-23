use indexmap::IndexMap;
use std::{
    ffi::CString,
    fs::{self, Metadata},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

use anyhow;

use crate::oid::Oid;

use super::Object;

const MODE: usize = 100644;
const MODE_EXECUTABLE: usize = 100755;
const MODE_DIR: usize = 40000;

#[derive(Debug, Clone)]
enum TreeNode {
    Leaf(Oid, Metadata),
    Branch(Tree),
}

#[derive(Debug, Clone)]
pub struct Tree {
    oid: Option<Oid>,
    entries: IndexMap<CString, TreeNode>,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            oid: None,
            entries: IndexMap::new(),
        }
    }

    pub fn traverse<F>(&mut self, f: &F) -> Result<(), anyhow::Error>
    where
        F: Fn(&mut Tree) -> Result<Oid, anyhow::Error>,
    {
        for (_name, entry) in self.entries.iter_mut() {
            if let TreeNode::Branch(ref mut tree) = entry {
                tree.traverse(f)?;
            }
        }

        f(self)?;

        Ok(())
    }

    pub fn add_entry(&mut self, path: PathBuf, oid: Oid) -> Result<(), anyhow::Error> {
        let components: Vec<_> = path
            .components()
            .map(|comp| comp.as_os_str().to_string_lossy().into_owned())
            .collect();

        let fmeta = fs::metadata(path)?;

        self.add_entry_recursive(&components, oid, fmeta);
        Ok(())
    }

    fn add_entry_recursive(&mut self, components: &[String], oid: Oid, stats: Metadata) {
        if components.is_empty() {
            return;
        }

        let component = &components[0];
        let fname = CString::new(component.as_bytes()).unwrap();

        if components.len() == 1 {
            self.entries
                .insert(fname.clone(), TreeNode::Leaf(oid, stats));
        } else {
            let entry = self
                .entries
                .entry(fname.clone())
                .or_insert_with(|| TreeNode::Branch(Tree::new()));

            if let TreeNode::Branch(ref mut tree) = entry {
                tree.add_entry_recursive(&components[1..], oid, stats);
            }
        }
    }

    fn is_executable(stat: &fs::Metadata) -> bool {
        stat.permissions().mode() & 0o111 != 0
    }

    fn mode(stat: &fs::Metadata) -> usize {
        if Tree::is_executable(stat) {
            MODE_EXECUTABLE
        } else {
            MODE
        }
    }

    fn serialize(name: &CString, tree_node: &TreeNode) -> Vec<u8> {
        let (oid, stats) = match tree_node {
            TreeNode::Leaf(o, m) => (o, Tree::mode(m)),
            TreeNode::Branch(t) => (&t.oid.unwrap(), MODE_DIR),
        };

        let mut serialized = Vec::new();
        serialized.extend_from_slice(stats.to_string().as_bytes());
        serialized.push(b' ');
        serialized.extend_from_slice(name.to_bytes_with_nul());
        serialized.extend_from_slice(oid.as_bytes());
        serialized
    }
}

impl Object for Tree {
    fn kind(&self) -> &[u8] {
        b"tree"
    }

    fn set_oid(&mut self, oid: Oid) {
        self.oid = Some(oid);
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.entries
            .iter()
            .flat_map(|(name, tree_node)| Tree::serialize(name, tree_node))
            .collect()
    }
}
