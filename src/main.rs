mod nt;
mod ui;

use std::sync::mpsc;

use log::LevelFilter;
use nt_client::{Client, NTAddr, NewClientOptions};

#[tokio::main]
async fn main() {
    let _ = simple_logging::log_to_file("test.log", LevelFilter::Info);
    // Create channel for NT updates
    let (sender, receiver) = mpsc::channel();

    // Create a tokio runtime
    let rt = tokio::runtime::Runtime::new().unwrap();

    let client = Client::new(NewClientOptions {
        addr: NTAddr::Local,
        // addr: NTAddr::Custom(Ipv4Addr::new(10.80.89.2)),
        ..Default::default()
    });

    let topics = vec![
        client.topic("/AdvantageKit/Timestamp"),
        client.topic("/AdvantageKit/RealOutputs/Drive/LeftPositionMeters"),
    ];
    // Spawn NT client task in the runtime
    let nt_handle = rt.spawn(nt::run_nt_client(sender, topics));

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
