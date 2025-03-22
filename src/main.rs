mod config;
mod nt;
mod ui;

use crate::ui::ConnectionStatus;
use log::{LevelFilter, error, info};
use nt_client::{NTAddr, NewClientOptions, error::ReconnectError};
use std::thread;
use std::time::Duration;
use tokio::sync::broadcast::{Sender, channel};

const RECONNECT_DELAY_MS: u64 = 2000;

#[tokio::main]
async fn main() {
    let _ = simple_logging::log_to_file("test.log", LevelFilter::Debug);

    // Create channel for NT updates
    let (sender, receiver) = channel(128);

    let client_opts = NewClientOptions {
        addr: NTAddr::Local, // Can be changed to custom address if needed
        ..Default::default()
    };

    // Start NT client with reconnection handling in a separate task

    let nt_task = tokio::spawn(run_nt_with_reconnect(sender.clone(), client_opts.clone()));

    // Run the UI with the receiver (this blocks the main thread)
    ui::run_ui(receiver).unwrap();
    // thread::sleep(Duration::from_secs(100));

    // When UI exits, abort all tasks
    nt_task.abort();
}

async fn run_nt_with_reconnect(sender: Sender<nt::NtUpdate>, client_opts: NewClientOptions) {
    // Run reconnect handler
    let _ = nt_client::reconnect(client_opts, |client| {
        // Create a new sender for this reconnection attempt
        let sender = sender.clone();
        async move {
            // Mark as connecting
            let _ = sender.send(nt::NtUpdate::ConnectionStatus(ConnectionStatus::Connecting));
            info!("Attempting to establish NT connection");

            let topics = client.topic("");
            let sender_c = sender.clone();
            let topics_c = topics.clone();
            tokio::spawn(nt::run_nt_client(sender_c.clone(), topics));
            tokio::spawn(nt::run_nt_client_topics(sender_c.clone(), topics_c));

            let recv = sender_c.clone().subscribe();
            let generic_publisher = client.generic_publisher();
            tokio::spawn(nt::run_nt_publisher(recv, generic_publisher));

            tokio::select! {
                conn_result = client.connect() => {
                    // Connection closed or errored
                    error!("NT connection closed: {:?}", conn_result);
                    let _ = sender.send(nt::NtUpdate::ConnectionStatus(ConnectionStatus::Disconnected));

                    // Return non-fatal error to trigger reconnect
                    thread::sleep(Duration::from_millis(RECONNECT_DELAY_MS));
                    match conn_result {
                        Ok(_) => Err(ReconnectError::Nonfatal("Connection closed".into())),
                        Err(e) => Err(ReconnectError::Nonfatal(e.into())),
                    }
                }
            }
        }
    })
    .await
    .unwrap_or_else(|e| {
        error!("Fatal NT connection error: {:?}", e);
    });
}
