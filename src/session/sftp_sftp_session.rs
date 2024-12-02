use std::collections::HashMap;
use std::os::unix::prelude::{MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};
use async_trait::async_trait;
use dotenv::dotenv;
use log::{error, info};
use russh_sftp::protocol::{Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode, Version};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use crate::utils::utils::create_root_dir;

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

        match create_root_dir(&root_dir).await {
            Ok(_) => {
                // todo("Have different ROOT_DIR for each SFTP client");
                self.server_root_dir = PathBuf::from(root_dir);
                self.cwd = self.server_root_dir.clone();
            },
            Err(e) => {
                eprintln!("Error creating root directory: {}", e);
                return Err(StatusCode::ConnectionLost);
            }
        }

        self.version = Some(version);
        println!("SftpSession::init: version: {:?} extensions: {:?}", self.version, extensions);


        Ok(Version::new())
    }

    async fn open(&mut self, id: u32, filename: String, pflags: OpenFlags, attrs: FileAttributes) -> Result<Handle, Self::Error> {
        println!("SftpSession::open: id: {:?} filename: {:?} pflags: {:?} attrs: {:?}", id, filename, pflags, attrs);
        let file_path = self.server_root_dir.join(&filename);
        let file = OpenOptions::new()
            .read(pflags.contains(OpenFlags::READ))
            .write(pflags.contains(OpenFlags::WRITE))
            .create(pflags.contains(OpenFlags::CREATE))
            .open(&file_path)
            .await
            .map_err(|_| StatusCode::PermissionDenied)?;

        Ok(Handle { id, handle: filename })
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
        println!("SftpSession::read: id: {:?} handle: {:?} offset: {:?} len: {:?}", id, handle, offset, len);
        todo!("Handle the read function properly");
    }

    async fn write(&mut self, id: u32, handle: String, offset: u64, data: Vec<u8>) -> Result<Status, Self::Error> {
        println!("SftpSession::write: id: {:?} handle: {:?} offset: {:?} data: {:?}", id, handle, offset, data);
        let file_path = self.server_root_dir.join(handle);

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&file_path)
            .await
            .map_err(|_| StatusCode::PermissionDenied)?;

        file.seek(tokio::io::SeekFrom::Start(offset)).await.map_err(|_| StatusCode::Failure)?;
        file.write_all(&data).await.map_err(|_| StatusCode::Failure)?;

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

        let path = if handle == "/" {
            self.server_root_dir.clone()
        } else {
            self.server_root_dir.join(handle)
        };

        let mut contents = tokio::fs::read_dir(path).await.map_err(|_| StatusCode::NoSuchFile)?;
        let mut files = Vec::new();

        while let Some(entry) = contents.next_entry().await.map_err(|_| StatusCode::Eof)? {
            let file_name = entry.file_name().into_string().map_err(|_| StatusCode::NoSuchFile)?;
            let metadata = entry.metadata().await.map_err(|_| StatusCode::Failure)?;

            let file_attrs = FileAttributes {
                size: Some(metadata.len()),
                uid: Some(metadata.uid()),
                gid: Some(metadata.gid()),
                permissions: Some(metadata.permissions().mode()),
                atime: Some(metadata.accessed().unwrap_or(UNIX_EPOCH).duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs() as u32),
                mtime: Some(metadata.modified().unwrap_or(UNIX_EPOCH).duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs() as u32),
                group: None,
                user: Some("SFTP-rustified".to_string()),
            };
            files.push(File::new(file_name,file_attrs));
        }

        if self.root_dir_read_done {
            return Err(StatusCode::Eof);
        }

        self.root_dir_read_done = true;

        Ok(Name {
            id,
            files
        })

        // if handle == "/" && self.root_dir_read_done {
        //     self.root_dir_read_done = false;
        //     return Ok(Name {
        //         id,
        //         files: vec![
        //             File::dummy("foo.txt"),
        //             File::dummy("bar.txt")
        //         ]
        //     })
        // }
        //
        // // If all files have been read, return an Err to indicate EOF not empty list
        // Err(StatusCode::Eof)
    }

    /// Used to Provide the Directory path to be used by the Client like "/"
    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        // todo!("Handle the realpath function properly");
        println!("SftpSession::realpath: id: {:?} path: {:?}", id, path);
        // let mut used_path ;
        // if path == "." {
        //     used_path = self.cwd.clone();
        // } else {
        //     used_path = self.cwd.join(path);
        // }
        // used_path.push("foo.txt");
        //
        // let mut file2 = tokio::fs::File::create(used_path).await.unwrap();
        // file2.write_all(b"Hello, World!").await.unwrap();
        //
        //
        //
        // Ok(Name {
        //     id,
        //     files: vec![
        //         File::new("foo.txt", FileAttributes::default()),
        //     ]
        // })

        Ok(Name {
            id,
            files: vec![
                File::new("/", FileAttributes::default()),
            ]
        })
    }


}