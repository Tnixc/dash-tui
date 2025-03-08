use crate::ui::ConnectionStatus;
use futures::future::join_all;
use log::info;
use nt_client::subscribe::ReceivedMessage;
use nt_client::topic::Topic;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum NtUpdate {
    KV(String, String),
    ConnectionStatus(ConnectionStatus),
}

// This function handles a single topic subscription
async fn handle_topic(sender: Sender<NtUpdate>, topic: Topic) {
    let mut subscriber = topic.subscribe(Default::default()).await;

    // If we're subscribing successfully, mark as connected
    let _ = sender.send(NtUpdate::ConnectionStatus(ConnectionStatus::Connected));

    loop {
        match subscriber.recv().await {
            Ok(ReceivedMessage::Announced(topic)) => {
                info!("Announced topic: {}", topic.name());
            }
            Ok(ReceivedMessage::Updated((topic, value))) => {
                let value = value.to_string().trim().to_string();
                let _ = sender.send(NtUpdate::KV(topic.name().to_string(), value));
            }
            Ok(ReceivedMessage::Unannounced { name, .. }) => {
                info!("Unannounced topic: {}", name);
            }
            Err(err) => {
                eprintln!("{err:?}");
                let _ = sender.send(NtUpdate::ConnectionStatus(ConnectionStatus::Disconnected));
                break;
            }
        }
    }
}

pub async fn run_nt_client(sender: Sender<NtUpdate>, topics: Vec<Topic>) {
    // Create a future for each topic
    let handlers = topics
        .into_iter()
        .map(|topic| {
            let sender_clone = sender.clone();
            handle_topic(sender_clone, topic)
        })
        .collect::<Vec<_>>();

    // Run all the handlers concurrently
    join_all(handlers).await;
}
