use std::collections::HashMap;
use async_trait::async_trait;
use log::{error, info};
use russh_sftp::protocol::{File, FileAttributes, Handle, Name, Status, StatusCode, Version};

#[derive(Default)]
pub struct SftpSession {
    version: Option<u32>,
    root_dir_read_done: bool
}

#[async_trait]
impl russh_sftp::server::Handler for SftpSession {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn init(&mut self, version: u32, extensions: HashMap<String, String>) -> Result<Version, Self::Error> {
        if self.version.is_some() {
            error!("SftpSession::init: version: {:?} extensions: {:?}", self.version, extensions);
            return Err(StatusCode::ConnectionLost);
        }

        self.version = Some(version);
        error!("SftpSession::init: version: {:?} extensions: {:?}", self.version, extensions);
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
                    File::new("foo", FileAttributes::default()),
                    File::new("bar", FileAttributes::default())
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