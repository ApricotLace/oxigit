use std::{
    env, fs,
    io::{self, Read},
    path::PathBuf,
};

use anyhow::Context;
use chrono::Local;
use db::Db;
use index::Index;
use object::{
    blob::Blob,
    commit::{self, Commit},
    tree::Tree,
};
use refs::Refs;
use workspace::Workspace;

pub mod db;
pub mod index;
pub mod object;
pub mod refs;
pub mod workspace;

pub struct ConfigUser {
    pub name: String,
    pub email: String,
}

pub struct Config {
    pub user: ConfigUser,
}

fn env_or_default(key: &str) -> String {
    env::var_os(key)
        .map(|var| var.to_string_lossy().to_string())
        .unwrap_or_default()
}

impl Config {
    fn from_env() -> Self {
        let name = env_or_default("GIT_AUTHOR_NAME");
        let email = env_or_default("GIT_AUTHOR_EMAIL");

        Self {
            user: ConfigUser { name, email },
        }
    }
}

pub struct Repository {
    root: PathBuf,
    workspace: Workspace,
    db: Db,
    refs: Refs,
    config: Config,
    index: Index,
}

impl Repository {
    pub fn open(path: PathBuf) -> Self {
        let workspace_path = path.clone();
        let root_path = path.join(".git");

        Self {
            root: path,
            workspace: Workspace::new(workspace_path),
            db: Db::new(root_path.clone()),
            refs: refs::Refs::new(root_path.clone()),
            config: Config::from_env(),
            index: Index::new(root_path.clone()),
        }
    }

    pub fn init(&self) -> Result<(), io::Error> {
        fs::create_dir(self.root.join(".git"))?;
        self.db.init()?;
        Ok(())
    }

    pub fn add(&mut self, path: PathBuf) -> Result<(), anyhow::Error> {
        let data = self.workspace.read_file(&path)?;
        let stats = self.workspace.stat_file(&path)?;

        let blob_oid = self.db.store_object(&mut Blob::new(data))?;

        self.index.add(path, blob_oid, stats)?;

        self.index.write_updates()?;

        Ok(())
    }

    pub fn commit(&self) -> Result<(), anyhow::Error> {
        let mut tree = Tree::new();

        for f in self.workspace.list_files()? {
            let data = self.workspace.read_file(&f)?;
            // TODO: use workspace.stat_file

            let blob_oid = self.db.store_object(&mut Blob::new(data))?;

            tree.add_entry(f, blob_oid)?;
        }

        tree.traverse(&|tree| self.db.store_object(tree))?;

        let tree_oid = self
            .db
            .store_object(&mut tree)
            .with_context(|| "Could not store tree")?;

        let parent = match self.refs.get_head() {
            Ok(head) => Some(head),
            Err(_) => None,
        };

        let name = &self.config.user.name;
        let email = &self.config.user.email;

        let author = commit::Author::new(name.to_owned(), email.to_owned(), Local::now());

        let mut commit_message = String::new();
        io::stdin().read_to_string(&mut commit_message)?;

        let mut commit = Commit::new(tree_oid, parent, author, commit_message.clone());

        let commit_oid = self
            .db
            .store_object(&mut commit)
            .with_context(|| "Could not store commit")?;

        self.refs.set_head(&commit_oid)?;

        let commit_message_fl = commit_message.lines().next().unwrap_or_default();

        let root_commit_marker = if parent.is_none() {
            "(root-commit) "
        } else {
            ""
        };

        println!(
            "[{}{}] {}",
            root_commit_marker, &commit_oid, commit_message_fl
        );

        Ok(())
    }
}
