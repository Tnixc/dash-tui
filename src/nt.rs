use crate::ui::ConnectionStatus;
use log::info;
use nt_client::subscribe::ReceivedMessage;
use nt_client::topic::collection::TopicCollection;
use nt_client::topic::{self, Topic};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum NtUpdate {
    KV(String, String),
    ConnectionStatus(ConnectionStatus),
    AvailableTopics(Vec<String>),
}

pub async fn run_nt_client(sender: Sender<NtUpdate>, topics: TopicCollection) {
    // Convert individual topics to a TopicCollection
    let mut subscriber = topics.subscribe(Default::default()).await;

    // If we're subscribing successfully, mark as connected
    let _ = sender.send(NtUpdate::ConnectionStatus(ConnectionStatus::Connected));

    // Collection of available topics
    let mut available_topics = Vec::new();

    // Process messages from all topics in the collection
    loop {
        match subscriber.recv().await {
            Ok(ReceivedMessage::Announced(topic)) => {
                let topic_name = topic.name().to_string();
                info!("Announced topic: {}", topic_name);

                // Add to available topics
                if !available_topics.contains(&topic_name) {
                    available_topics.push(topic_name);
                    // Send updated list of topics
                    let _ = sender.send(NtUpdate::AvailableTopics(available_topics.clone()));
                }
            }
            Ok(ReceivedMessage::Updated((topic, value))) => {
                let value = value.to_string().trim().to_string();
                let _ = sender.send(NtUpdate::KV(topic.name().to_string(), value));
            }
            Ok(ReceivedMessage::Unannounced { name, .. }) => {
                info!("Unannounced topic: {}", name);

                // Remove from available topics
                if let Some(index) = available_topics.iter().position(|t| t == &name) {
                    available_topics.remove(index);
                    // Send updated list of topics
                    let _ = sender.send(NtUpdate::AvailableTopics(available_topics.clone()));
                }
            }
            Err(err) => {
                eprintln!("{err:?}");
                let _ = sender.send(NtUpdate::ConnectionStatus(ConnectionStatus::Disconnected));
                break;
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
