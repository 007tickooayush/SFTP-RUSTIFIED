use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};
use async_trait::async_trait;
use dotenv::dotenv;
use log::{error, info};
use russh_sftp::protocol::{Data, File, FileAttributes, Handle, Name, Status, StatusCode, Version};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

#[derive(Default)]
pub struct SftpSession {
    version: Option<u32>,
    root_dir_read_done: bool,
    server_root_dir: PathBuf,
    cwd:PathBuf,
}

#[async_trait]
impl russh_sftp::server::Handler for SftpSession {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn init(&mut self, version: u32, extensions: HashMap<String, String>) -> Result<Version, Self::Error> {
        dotenv().ok();

        info!("SftpSession::init: version: {:?} extensions: {:?}", version, extensions);
        if self.version.is_some() {
            error!("SftpSession::init: version: {:?} extensions: {:?}", self.version, extensions);
            return Err(StatusCode::ConnectionLost);
        }

        let root_dir = std::env::var("ROOT_DIR").unwrap_or(".".to_string());
        self.server_root_dir = PathBuf::from(root_dir);
        self.cwd = self.server_root_dir.clone();

        self.version = Some(version);
        println!("SftpSession::init: version: {:?} extensions: {:?}", self.version, extensions);



        Ok(Version::new())
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        // todo!("Handle the close function properly");
        Ok(Status {
            id,
            status_code: StatusCode::Ok,
            error_message: "Ok".to_string(),
            language_tag: "en-US".to_string()
        })
    }

    async fn read(&mut self, id: u32, handle: String, offset: u64, len: u32) -> Result<Data, Self::Error> {
        todo!("Handle the read function properly");
    }

    async fn write(&mut self, id: u32, handle: String, offset: u64, data: Vec<u8>) -> Result<Status, Self::Error> {
        todo!("Handle the write function properly");
    }



    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, Self::Error> {
        println!("SftpSession::opendir: id: {:?} path: {:?}", id, path);
        self.root_dir_read_done = false;
        Ok(Handle { id, handle: path })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        // todo!("Handle the readdir function properly");
        println!("SftpSession::readdir: id: {:?} handle: {:?}", id, handle);
        if handle == "/" && self.root_dir_read_done {
            self.root_dir_read_done = false;
            return Ok(Name {
                id,
                files: vec![
                    File::new("foo.txt", FileAttributes::default()),
                    File::new("bar.txt", FileAttributes::default())
                ]
            })
        }

        // If all files have been read, return an Err to indicate EOF not empty list
        Err(StatusCode::Eof)
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        // todo!("Handle the realpath function properly");
        println!("SftpSession::realpath: id: {:?} path: {:?}", id, path);
        Ok(Name {
            id,
            files: vec![
                File::dummy("/")
            ]
        })
    }


}