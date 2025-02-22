use crate::oid::Oid;

use chrono::{DateTime, FixedOffset, TimeZone};

#[derive(Debug, Clone)]
pub struct Author {
    name: String,
    email: String,
    a_time: DateTime<FixedOffset>,
}

impl Author {
    pub fn new<Tz: TimeZone>(name: String, email: String, atime: DateTime<Tz>) -> Self {
        Self {
            a_time: atime.fixed_offset(),
            name,
            email,
        }
    }

    pub fn string(&self) -> String {
        let unix_timestamp = self.a_time.timestamp();
        let utc_offset = self.a_time.fixed_offset().format("%z");

        format!(
            "{} <{}> {} {}",
            self.name, self.email, unix_timestamp, utc_offset
        )
    }
}

#[derive(Debug, Clone)]
pub struct Commit {
    tree: Oid,
    parent: Option<Oid>,
    author: Author,
    message: String,
}

impl Commit {
    pub fn new(tree_oid: Oid, parent: Option<Oid>, author: Author, message: String) -> Self {
        Self {
            tree: tree_oid,
            parent,
            author,
            message,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        format!(
            "tree {}\n{}author {}\ncommiter {}\n\n{}",
            self.tree,
            match self.parent {
                Some(parent) => format!("parent {}\n", parent),
                None => String::new(),
            },
            self.author.string(),
            self.author.string(),
            self.message
        )
        .into()
    }
}
