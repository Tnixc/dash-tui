mod nt;
mod ui;

use std::sync::mpsc;

use nt_client::{Client, NTAddr, NewClientOptions};
use tracing::level_filters::LevelFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();
    // Create channel for NT updates
    let (sender, receiver) = mpsc::channel();

    // Create a tokio runtime
    let rt = tokio::runtime::Runtime::new().unwrap();

    let client = Client::new(NewClientOptions {
        addr: NTAddr::Local,
        // custom WSL ip
        // addr: NTAddr::Custom(Ipv4Addr::new(172, 30, 64, 1)),
        ..Default::default()
    });

    let sub_topic = client.topic("/AdvantageKit/Timestamp");
    // Spawn NT client task in the runtime
    let nt_handle = rt.spawn(nt::run_nt_client(sender, sub_topic));
    
    // Spawn a thread to run the client connection which is blocking
    tokio::spawn(async move {
        client.connect().await.unwrap();
    });
    // Run the UI with the receiver (this blocks the main thread)
    ui::run_ui(receiver).unwrap();

    // When UI exits, abort the NT task
    rt.block_on(async {
        nt_handle.abort();
    });
}
