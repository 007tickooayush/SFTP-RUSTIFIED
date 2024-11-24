use std::net::SocketAddr;
use crate::session::sftp_ssh_session::SshSession;

#[derive(Clone)]
pub struct SftpServer;

impl russh::server::Server for SftpServer {
    type Handler = SshSession;

    fn new_client(&mut self, _peer_addr: Option<SocketAddr>) -> Self::Handler {
        SshSession::default()
    }
}