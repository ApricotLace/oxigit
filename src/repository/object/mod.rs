use blob::Blob;
use commit::Commit;
use tree::Tree;

use crate::oid::Oid;

pub mod blob;
pub mod commit;
pub mod tree;

#[derive(Debug, Clone)]
pub struct DbObject {
    data: Vec<u8>,
    oid: Oid,
}

impl DbObject {
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn oid(&self) -> &Oid {
        &self.oid
    }
}

impl From<Object> for DbObject {
    fn from(value: Object) -> Self {
        let (kind, contents) = match value {
            Object::Blob(blob) => (b"blob".as_slice(), blob.as_bytes().to_owned()),
            Object::Commit(commit) => (b"commit".as_slice(), commit.to_bytes()),
            Object::Tree(tree) => (b"tree".as_slice(), tree.to_bytes()),
        };

        let mut content: Vec<u8> = Vec::new();
        content.extend_from_slice(kind);
        content.push(b' ');
        content.extend_from_slice(contents.len().to_string().as_bytes());
        content.push(0);
        content.extend_from_slice(&contents);

        let oid = Oid::new(&content);

        Self {
            data: content,
            oid: oid,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Object {
    Commit(Commit),
    Tree(Tree),
    Blob(Blob),
}

impl From<Commit> for Object {
    fn from(value: Commit) -> Self {
        Self::Commit(value)
    }
}

impl From<Tree> for Object {
    fn from(value: Tree) -> Self {
        Self::Tree(value)
    }
}

impl From<Blob> for Object {
    fn from(value: Blob) -> Self {
        Self::Blob(value)
    }
}
