mod config;
mod nt;
mod ui;

use crate::ui::ConnectionStatus;
use log::{LevelFilter, error, info};
use nt_client::{NTAddr, NewClientOptions, error::ReconnectError};
use std::str::FromStr;
use std::time::Duration;
use std::{net::Ipv4Addr, thread};
use tokio::sync::broadcast::{Sender, channel};

#[tokio::main]
async fn main() {
    let arg = std::env::args().nth(1);
    match arg {
        Some(arg) => match arg.as_str() {
            "--address" => {}
            _ => {
                println!("Invalid argument: {}. Valid arguments: --address", arg);
                std::process::exit(1);
            }
        },
        None => {
            println!("No argument given. Valid arguments: --address");
            std::process::exit(1);
        }
    }
    let addr_arg = std::env::args().nth(2);
    if addr_arg.is_none() {
        println!("No arguments passed to --address");
        std::process::exit(1);
    }

    let addr_arg = addr_arg.unwrap();
    let addr = match addr_arg.parse::<u16>() {
        Ok(n) => NTAddr::TeamNumber(n),
        Err(_) => {
            let a = Ipv4Addr::from_str(addr_arg.as_str());
            match a {
                Ok(ip) => NTAddr::Custom(ip),
                Err(e) => {
                    if addr_arg == "localhost" {
                        NTAddr::Local
                    } else {
                        eprintln!("Invalid address: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    };
    let _ = simple_logging::log_to_file("test.log", LevelFilter::Debug);

    // Create channel for NT updates
    let (sender, receiver) = channel(128);

    let client_opts = NewClientOptions {
        addr, // Can be changed to custom address if needed
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
    nt_client::reconnect(client_opts, |client| {
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
                    thread::sleep(Duration::from_millis(2000));
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
