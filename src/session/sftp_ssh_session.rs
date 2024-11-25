use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use russh::{Channel, ChannelId};
use russh::server::{Auth, Handler, Msg, Session};
use log::info;
use russh_keys::key::PublicKey;
use crate::session::sftp_sftp_session::SftpSession;

pub struct SshSession {
    connections: Arc<Mutex<HashMap<ChannelId, Channel<Msg>>>>
}

impl Default for SshSession {
    fn default() -> Self {
        // todo!("Handle the users in full fledged manned via DB or local files");
        Self {
            connections: Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

impl SshSession {
    pub async fn get_channel(&mut self, channel_id: ChannelId) -> Channel<Msg> {
        let mut connections = self.connections.lock().unwrap();
        connections.remove(&channel_id).unwrap()
    }
}


/// NOTE: mind the imports as here the import is from `russh::server::Handler`
/// and using async_trait from async-trait crate to handle the async functions/members and their lifetimes
#[async_trait]
impl Handler for SshSession {
    type Error = anyhow::Error;

    async fn auth_password(&mut self, user: & str, password: & str) -> Result<Auth, Self::Error> {
        // todo!("Implement proper user authentication");
        println!("SshSession::auth_password: User: {} Password: {}", user, password);
        if user == "master" && password == "master" {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::Reject { proceed_with_methods: None })
        }
    }

    async fn auth_publickey(&mut self, user: &str, public_key: &PublicKey) -> Result<Auth, Self::Error> {
        // todo!("Implement Proper Public Key Authentication");
        println!("SshSession::auth_publickey: User: {} Public Key: {:?}", user, public_key);
        Ok(Auth::Accept)
    }

    async fn channel_eof(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        // Close the channel after the EOF has been received from the client connection
        session.close(channel);
        Ok(())
    }

    async fn channel_open_session(&mut self, channel: Channel<Msg>, session: &mut Session) -> Result<bool, Self::Error> {
        // todo!("Handle the channel open session Properly");
        {
            let mut connections = self.connections.lock().unwrap();
            connections.insert(channel.id(), channel);
        }
        Ok(true)
    }

    async fn subsystem_request(&mut self, channel_id: ChannelId, name: &str, session: &mut Session) -> Result<(), Self::Error> {
        // todo!("Handle the subsystem request properly");
        println!("Subsystem Request: {}", name);
        if name == "sftp" {
            let channel = self.get_channel(channel_id).await;
            let sftp = SftpSession::default();
            session.channel_success(channel_id);
            russh_sftp::server::run(channel.into_stream(), sftp).await;
        }

        Ok(())
    }
}