use std::io;
use std::path::PathBuf;

pub trait FileHandler {
    fn complete_path(&self, path: PathBuf) -> Result<PathBuf, io::Error>;
    fn remove_prefix_str(&self, path: PathBuf, prefix:&str) -> Result<PathBuf, io::Error>;
}