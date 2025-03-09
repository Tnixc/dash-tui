use crate::ui::ConnectionStatus;
use log::error;
use log::info;
use log::warn;
use nt_client::NewClientOptions;
use nt_client::data::SubscriptionOptions;
use nt_client::subscribe::ReceivedMessage;
use nt_client::topic::collection::TopicCollection;
use nt_client::topic::{AnnouncedTopic, Topic};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub enum NtUpdate {
    KV(String, String),
    ConnectionStatus(ConnectionStatus),
    AvailableTopics(Vec<String>),
}

pub async fn run_nt_client(sender: Sender<NtUpdate>, topics: Topic) {
    // Convert individual topics to a TopicCollection
    let mut subscriber = topics
        .subscribe(SubscriptionOptions {
            prefix: Some(true),
            ..Default::default()
        })
        .await;

    // If we're subscribing successfully, mark as connected
    let _ = sender.send(NtUpdate::ConnectionStatus(ConnectionStatus::Connected));

    // Collection of available topics

    // Process messages from all topics in the collection
    loop {
        match subscriber.recv_latest().await {
            Ok(ReceivedMessage::Announced(topic)) => {
                let topic_name = topic.name().to_string();
                info!("Announced topic: {}", topic_name);
                let _ = sender.send(NtUpdate::KV(topic.name().to_string(), "None".to_owned()));
            }
            Ok(ReceivedMessage::Updated((topic, value))) => {
                let value = value.to_string().trim().to_string();
                let _ = sender.send(NtUpdate::KV(topic.name().to_string(), value));
            }
            Ok(ReceivedMessage::Unannounced { name, .. }) => {
                info!("Unannounced topic: {}", name);
            }
            Err(err) => {
                warn!("Warning on specific watcher thread: {err:?}");
            }
        }
    }
}
