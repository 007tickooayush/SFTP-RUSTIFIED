use async_trait::async_trait;
use russh_sftp::protocol::StatusCode;

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
}