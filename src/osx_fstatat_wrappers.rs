use std::fs::DirEntry as DirEntryInternal;
use std::path::{Path as PathInternal, PathBuf as PathBufInternal};
use std::fs::Metadata;

pub struct DirEntry(DirEntryInternal);
pub struct Path(&'a PathInternal);
pub struct PathBuf(PathBufInternal);

impl<'a> Path<'a> {
    pub fn new(string: &str) -> Path {
        Path(PathInternal::new(string))
    }

    pub fn metadata(&self, follow_symlink: bool) -> std::io::Result<Metadata> {
        if follow_symlink {
            self.0.metadata()
        } else {
            self.0.symlink_metadata()
        }
    }

    pub fn read_dir(&self) -> Vec<DirEntry> {
        self.0.read_dir().unwrap().map(|e| {
            DirEntry(e.unwrap())
        }).collect::<Vec<DirEntry>>()
    }

    pub fn to_str(&self) -> Option<&str> {
        self.0.to_str()
    }
}

impl DirEntry {
    pub fn metadata(&self, follow_symlink: bool) -> std::io::Result<Metadata> {
        if follow_symlink {
            self.0.path().metadata()
        } else {
            self.0.metadata()
        }
    }

    pub fn read_dir(&self) -> Vec<DirEntry> {
        self.0.path().read_dir().unwrap().map(|e| {
            DirEntry(e.unwrap())
        }).collect::<Vec<DirEntry>>()
    }

    pub fn path(&self) -> PathBuf {
        PathBuf(self.0.path())
    }
}

impl PathBuf {
    pub fn to_str(&self) -> Option<&str> {
        self.0.to_str()
    }
}

#[cfg(test)]
mod test {
    mod normal_wrappers;
}