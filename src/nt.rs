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

pub async fn run_nt_client(sender: Sender<NtUpdate>, topics: TopicCollection) {
    // Convert individual topics to a TopicCollection
    let mut subscriber = topics
        .subscribe(SubscriptionOptions {
            ..Default::default()
        })
        .await;

    // If we're subscribing successfully, mark as connected
    let _ = sender.send(NtUpdate::ConnectionStatus(ConnectionStatus::Connected));

    // Collection of available topics

    // Process messages from all topics in the collection
    loop {
        match subscriber.recv().await {
            Ok(ReceivedMessage::Announced(topic)) => {
                let topic_name = topic.name().to_string();
                info!("Announced topic: {}", topic_name);
            }
            Ok(ReceivedMessage::Updated((topic, value))) => {
                let value = value.to_string().trim().to_string();
                let _ = sender.send(NtUpdate::KV(topic.name().to_string(), value));
            }
            Ok(ReceivedMessage::Unannounced { name, .. }) => {
                info!("Unannounced topic: {}", name);
            }
            Err(err) => {
                error!("Error: {err:?}");
            }
        }
    }
}

// Helper function to add a new topic to the subscription list
pub async fn subscribe_to_topic(
    client: &nt_client::Client,
    sender: Sender<NtUpdate>,
    topic_name: String,
) {
    // Create a new topic and subscribe to it
    let topic = client.topic(&topic_name);
    let mut subscriber = topic.subscribe(Default::default()).await;

    // Process messages from this specific topic
    tokio::spawn(async move {
        loop {
            match subscriber.recv().await {
                Ok(ReceivedMessage::Updated((topic, value))) => {
                    let value = value.to_string().trim().to_string();
                    let _ = sender.send(NtUpdate::KV(topic.name().to_string(), value));
                }
                Err(_) => break,
                _ => {}
            }
        }
    });
}

pub async fn get_available_topics(sender: Sender<NtUpdate>, sub_topic: Topic) {
    let mut subscriber = sub_topic
        .subscribe(SubscriptionOptions {
            prefix: Some(true),
            topics_only: Some(true),
            ..Default::default()
        })
        .await;

    // Process the received messages in the loop without cloning sender each time
    loop {
        info!("****************************************");
        match subscriber.recv().await {
            Ok(ReceivedMessage::Announced(topic)) => {
                info!("1111111111111111111111111111111111111111");
                let topic_name = topic.name().to_string();
                info!("Announced topic: {}", topic_name);
            }
            Ok(ReceivedMessage::Unannounced { name, .. }) => {
                info!("2222222222222222222222222222222222222222");
                info!("Unannounced topic: {}", name);
            }
            Err(e) => {
                info!("3333333333333333333333333333333333333333");
                warn!("Blocking task: {}", e);
            }
            Ok(ReceivedMessage::Updated((topic, value))) => {
                info!("4444444444444444444444444444444444444444");
                let value = value.to_string().trim().to_string();
                sender.send(NtUpdate::KV(topic.name().to_string(), value));
            }
        }
    }
}
