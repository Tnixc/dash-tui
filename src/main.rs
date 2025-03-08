mod nt;
mod ui;

use std::sync::mpsc;
use std::time::Duration;

use crate::ui::ConnectionStatus;
use log::{LevelFilter, error, info};
use nt_client::{Client, NTAddr, NewClientOptions};

#[tokio::main]
async fn main() {
    let _ = simple_logging::log_to_file("test.log", LevelFilter::Info);
    // Create channel for NT updates
    let (sender, receiver) = mpsc::channel();

    // Create a tokio runtime
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Start the connection handling task
    let connection_handle = rt.spawn(manage_nt_connection(sender.clone()));

    // Run the UI with the receiver (this blocks the main thread)
    ui::run_ui(receiver).unwrap();

    // When UI exits, abort all tasks
    rt.block_on(async {
        connection_handle.abort();
    });
}

async fn manage_nt_connection(sender: mpsc::Sender<nt::NtUpdate>) {
    loop {
        // Mark as disconnected at the start of each connection attempt
        let _ = sender.send(nt::NtUpdate::ConnectionStatus(
            ConnectionStatus::Disconnected,
        ));

        info!("Establishing NT connection");

        let client_opts = NewClientOptions {
            addr: NTAddr::Local,
            // addr: NTAddr::Custom(Ipv4Addr::new(10.80.89.2)),
            ..Default::default()
        };
        let client = Client::new(client_opts);

        let topics = vec![
            client.topic("/AdvantageKit/Timestamp"),
            client.topic("/AdvantageKit/RealOutputs/Drive/LeftPositionMeters"),
        ];

        // Spawn NT client task
        let nt_handle = tokio::spawn(nt::run_nt_client(sender.clone(), topics));

        // Try to establish the connection
        let connection_result = client.connect().await;

        if let Err(err) = connection_result {
            error!("NT connection error: {:?}", err);
            let _ = sender.send(nt::NtUpdate::ConnectionStatus(
                ConnectionStatus::Disconnected,
            ));
        } else {
            error!("NT connection closed unexpectedly");
            let _ = sender.send(nt::NtUpdate::ConnectionStatus(
                ConnectionStatus::Disconnected,
            ));
        }

        // Abort the NT handler task
        nt_handle.abort();

        // Wait before attempting to reconnect
        info!("Waiting 500ms before reconnection attempt");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
