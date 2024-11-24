use std::sync::Arc;
use dotenv::dotenv;
use log::info;
use russh::server::Server;
use russh_keys::key::KeyPair;
use crate::server::server_sftp::SftpServer;

mod server;
mod session;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // initial configuration for the server
    let config = russh::server::Config {
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        keys: vec![KeyPair::generate_ed25519()],
        ..Default::default()
    };

    // COMPLETE the server configuration and run the server
    let mut server = SftpServer;

    let address = "0.0.0.0";
    let port = std::env::var("PORT").unwrap_or("2002".to_string()).parse().unwrap(); // do not use PORT 22 in local development
    info!("Main::main: address:port => {}:{}", address, port);

    server.run_on_address(
        Arc::new(config),
        (
            address,
            port,
        ),
    )
        .await
        .unwrap();

}
