use std::collections::HashMap;
use std::io;
use std::io::{Error, ErrorKind};
use std::os::unix::prelude::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use async_trait::async_trait;
use dotenv::dotenv;
use log::{error, info};
use russh_sftp::protocol::{Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode, Version};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use crate::utils::file_handler_util::FileHandler;
use crate::utils::utils::create_root_dir;

#[derive(Default)]
pub struct SftpSession {
    version: Option<u32>,
    root_dir_read_done: bool,
    server_root_dir: PathBuf,
    cwd: PathBuf,
}

#[async_trait]
impl russh_sftp::server::Handler for SftpSession {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn init(&mut self, version: u32, extensions: HashMap<String, String>) -> Result<Version, Self::Error> {
        dotenv().ok();
        println!("SftpSession::init: version: {:?} extensions: {:?}", version, extensions);
        if self.version.is_some() {
            error!("SftpSession::init: version: {:?} extensions: {:?}", self.version, extensions);
            return Err(StatusCode::ConnectionLost);
        }

        let root_dir = std::env::var("ROOT_DIR").unwrap_or(".".to_string());
        match create_root_dir(&root_dir).await {
            Ok(_) => {
                self.server_root_dir = PathBuf::from(root_dir);
                self.cwd = PathBuf::from("/");
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
        let file_path = self.server_root_dir.join(filename.trim_start_matches("/"));
        if !file_path.exists() {
            // let file = tokio::fs::File::create(&file_path).await.map_err(|_| StatusCode::PermissionDenied)?;
            OpenOptions::new()
                .create_new(true)
                .read(pflags.contains(OpenFlags::READ))
                .write(pflags.contains(OpenFlags::WRITE))
                .open(&file_path)
                .await
                .map_err(|_| StatusCode::PermissionDenied)?;
        } else {
            OpenOptions::new()
                .read(pflags.contains(OpenFlags::READ))
                .write(pflags.contains(OpenFlags::WRITE))
                .open(&file_path)
                .await
                .map_err(|_| StatusCode::PermissionDenied)?;
        }

        Ok(Handle { id, handle: file_path.to_string_lossy().to_string() })
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        println!("SftpSession::close: id: {:?} handle: {:?}", id, handle);
        // Implement the close logic if needed
        Ok(Status {
            id,
            status_code: StatusCode::Ok,
            error_message: "Ok".to_string(),
            language_tag: "en-US".to_string()
        })
    }

    async fn read(&mut self, id: u32, handle: String, offset: u64, len: u32) -> Result<Data, Self::Error> {
        println!("SftpSession::read: id: {:?} handle: {:?} offset: {:?} len: {:?}", id, handle, offset, len);
        let file_path = self.server_root_dir.join(handle);
        let mut file = OpenOptions::new().read(true).open(&file_path).await.map_err(|_| StatusCode::NoSuchFile)?;

        file.seek(tokio::io::SeekFrom::Start(offset)).await.map_err(|_| StatusCode::Failure)?;
        let mut buffer = vec![0; len as usize];
        let n = file.read(&mut buffer).await.map_err(|_| StatusCode::Failure)?;

        Ok(Data { id, data: buffer[..n].to_vec() })
    }

    async fn write(&mut self, id: u32, handle: String, offset: u64, data: Vec<u8>) -> Result<Status, Self::Error> {
        println!("SftpSession::write: id: {:?} handle: {:?} offset: {:?} data: {:?}", id, handle, offset, data);
        let file_path = self.server_root_dir.join(handle);
        let mut file = OpenOptions::new().write(true).open(&file_path).await.map_err(|_| StatusCode::PermissionDenied)?;

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
        // let mut path = self.server_root_dir.to_string_lossy().to_string().push(');
        // let mut return_path = self.server_root_dir.clone();
        // return_path.push(path.trim_start_matches('/'));
        // let return_path = return_path.to_string_lossy().to_string();
        // self.cwd = PathBuf::from(return_path.clone());
        Ok(Handle { id, handle: path })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        println!("SftpSession::readdir: id: {:?} handle: {:?}", id, handle);
        let path = if handle == "/" {
            self.server_root_dir.clone()
        } else {
            self.server_root_dir.join(handle)
        };

        let mut contents = tokio::fs::read_dir(path).await.map_err(|_| StatusCode::NoSuchFile)?;
        let mut files = Vec::new();

        while let Some(entry) = contents.next_entry().await.map_err(|_| StatusCode::Failure)? {
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
            files.push(File::new(file_name, file_attrs));
        }

        if self.root_dir_read_done {
            return Err(StatusCode::Eof);
        }

        self.root_dir_read_done = true;

        Ok(Name { id, files })
    }

    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, Self::Error> {
        println!("SftpSession::remove: id: {:?} filename: {:?}", id, filename);
        let file_path = self.server_root_dir.join(filename.trim_start_matches("/"));
        // .map_err(|_| StatusCode::PermissionDenied)?;
        match tokio::fs::remove_file(&file_path).await {
            Ok(_) => {
                Ok(Status {
                    id,
                    status_code: StatusCode::Ok,
                    error_message: "Ok".to_string(),
                    language_tag: "en-US".to_string()
                })
            },
            Err(e) => {
                eprintln!("Error removing file: {}", e);
                Err(StatusCode::PermissionDenied)
            }
        }
    }
    async fn mkdir(&mut self, id: u32, path: String, attrs: FileAttributes) -> Result<Status, Self::Error> {
        println!("SftpSession::mkdir: id: {:?} path: {:?} attrs: {:?}", id, path, attrs);
        let path = self.server_root_dir.join(path.trim_start_matches("/"));
        match tokio::fs::create_dir(&path).await {
            Ok(_) => {
                Ok(Status {
                    id,
                    status_code: StatusCode::Ok,
                    error_message: "Ok".to_string(),
                    language_tag: "en-US".to_string()
                })
            },
            Err(_) => {
                Err(StatusCode::PermissionDenied)
            }
        }
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        println!("SftpSession::rmdir: id: {:?} path: {:?}", id, path);
        let dir_path = self.server_root_dir.join(path.trim_start_matches("/"));

        if dir_path.is_dir() {
            match tokio::fs::remove_dir(&dir_path).await {
                Ok(_) => {
                    Ok(Status {
                        id,
                        status_code: StatusCode::Ok,
                        error_message: "Ok".to_string(),
                        language_tag: "en-US".to_string()
                    })
                },
                Err(e) => {
                    eprintln!("Error removing directory: {}", e);
                    Err(StatusCode::PermissionDenied)
                }
            }
        } else {
            eprintln!("Error removing directory(NOT A DIRECTORY): {:?}", dir_path);
            Err(StatusCode::OpUnsupported)
        }
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        println!("SftpSession::realpath: id: {:?} path: {:?}", id, path);
        let new_path = PathBuf::from(path.clone()).canonicalize().map_err(|_| StatusCode::NoSuchFile)?;
        println!("SftpSession::realpath: new_path: {:?}", new_path);
        // self.cwd = ;
        let path = PathBuf::from(path).canonicalize().map_err(|_| StatusCode::NoSuchFile)?.to_string_lossy().to_string();
        // let return_path = self.cwd.to_str().ok_or_else(|| StatusCode::NoSuchFile)?;
        Ok(Name {
            id,
            files: vec![File::new("/", FileAttributes::default())]
        })
    }
}

impl FileHandler for SftpSession {
    fn complete_path(&self, path: PathBuf) -> Result<PathBuf, io::Error> {
        let mut path = path;
        let path: PathBuf = path.components().filter(|c| *c != std::path::Component::CurDir).collect();
        let mut directory = self.server_root_dir.join( if path.has_root() {
            // let path = path.components().filter(|c| *c != std::path::Component::CurDir).collect();
            path.iter().skip(1).collect()
        } else {
            path
        });

        // Normalize the path by removing "." segments
        // directory = directory.components().filter(|c| *c != std::path::Component::CurDir).collect();

        let dir = directory.canonicalize();
        if let Ok(ref dir) = dir {
            if !dir.starts_with(&self.server_root_dir) {
                return Err(ErrorKind::PermissionDenied.into());
            }
        }
        dir
    }

    fn remove_prefix_str(&self, path: PathBuf, prefix: &str) -> Result<PathBuf, Error> {
        if path.starts_with(prefix.to_string()) {
            // let path = path.strip_prefix(prefix).map_err(|_| Error::new(ErrorKind::InvalidInput, "Path does not start with prefix"))?.to_path_buf();
            // Ok(path)
            match path.strip_prefix(prefix) {
                Ok(path) => Ok(Path::new(path).to_path_buf()),
                Err(_) => Err(Error::new(ErrorKind::InvalidInput, "Path does not start with prefix"))
            }
        } else {
            Err(Error::new(ErrorKind::InvalidInput, "Path does not start with prefix"))
        }
    }
}