use std::io;
use std::path::PathBuf;

pub trait FileHandler {
    fn complete_path(&self, path: PathBuf) -> Result<PathBuf, io::Error>;
}