mod config;
mod fuzzy;
mod nt;
mod ui;

use crate::ui::ConnectionStatus;
use log::{LevelFilter, error, info};
use nt_client::{NTAddr, NewClientOptions, error::ReconnectError};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const RECONNECT_DELAY_MS: u64 = 2000;

#[tokio::main]
async fn main() {
    let _ = simple_logging::log_to_file("test.log", LevelFilter::Debug);

    // Create channel for NT updates
    let (sender, receiver) = mpsc::channel();

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

async fn run_nt_with_reconnect(sender: mpsc::Sender<nt::NtUpdate>, client_opts: NewClientOptions) {
    // Run reconnect handler
    let _ = nt_client::reconnect(client_opts, |client| {
        // Create a new sender for this reconnection attempt
        let sender = sender.clone();
        async move {

            // FIXME: initialize this elsewhere
            let initial_topics = vec![
                "/AdvantageKit/Timestamp",
    "/AdvantageKit/RealOutputs/Logger/AutoLogMS",
    "/AdvantageKit/SystemStats/CANBus/ReceiveErrorCount",
    "/AdvantageKit/RealOutputs/LoggedRobot/FullCycleMS",
    "/AdvantageKit/DriverStation/Joystick1/POVs",
    "/AdvantageKit/RealOutputs/Logger/DashboardInputsMS",
            ].to_owned();


            // Mark as connecting
            let _ = sender.send(nt::NtUpdate::ConnectionStatus(ConnectionStatus::Connecting));
            info!("Attempting to establish NT connection");

            // Create topics collection for initial topics
            let topics = client.topics(initial_topics.iter().map(|name| name.to_string()).collect());
            let topic_sender = sender.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("failed to start thread for TOPIC");
                rt.block_on(async {
                    nt::run_nt_client(topic_sender, topics).await;
                });
            });

            // Start NT client handler that processes messages
            let all = client.topic("/");
            let all_sender = sender.clone();
            let all_clone = all.clone();
            // Move heavy topic processing to a dedicated thread to avoid lagging main tasks
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("failed to start thread for ALL");
                rt.block_on(async {
                    nt::get_available_topics(all_sender, all_clone).await;
                });
            });

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
