use crate::ui::ConnectionStatus;
use log::error;
use log::info;
use log::warn;
use nt_client::data::SubscriptionOptions;
use nt_client::publish::GenericPublisher;
use nt_client::subscribe::ReceivedMessage;
use nt_client::topic::Topic;
use rmpv::Value;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::Sender;
#[derive(Debug, Clone)]

pub enum NtUpdate {
    Subscribed(String, String),
    Publish(String, Value),
    ConnectionStatus(ConnectionStatus),
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

    // Process messages from all topics in the collection
    loop {
        match subscriber.recv().await {
            Ok(ReceivedMessage::Announced(topic)) => {
                let topic_name = topic.name().to_string();
                info!("Announced topic: {}", topic_name);
                let _ = sender.send(NtUpdate::Subscribed(
                    topic.name().to_string(),
                    "None".to_owned(),
                ));
            }
            Ok(ReceivedMessage::Updated((topic, value))) => {
                let value = value.to_string().trim().to_string();
                let _ = sender.send(NtUpdate::Subscribed(topic.name().to_string(), value));
            }
            Err(err) => {
                warn!("Warning on specific watcher thread: {err:?}");
            }
            _ => {}
        }
    }
}

pub async fn run_nt_client_topics(sender: Sender<NtUpdate>, topics: Topic) {
    let mut subscriber = topics
        .subscribe(SubscriptionOptions {
            prefix: Some(true),
            all: Some(true),
            topics_only: Some(true),
            ..Default::default()
        })
        .await;
    loop {
        match subscriber.recv_buffered().await {
            Ok(ReceivedMessage::Announced(topic)) => {
                let topic_name = topic.name().to_string();
                info!("Announced topic: {}", topic_name);
                let _ = sender.send(NtUpdate::Subscribed(
                    topic.name().to_string(),
                    "None".to_owned(),
                ));
            }
            Ok(ReceivedMessage::Unannounced { name, .. }) => {
                info!("Unannounced topic: {}", name);
            }
            Err(err) => {
                warn!("Warning on topics thread: {err:?}");
            }
            _ => {}
        }
    }
}

pub async fn run_nt_publisher(
    mut receiver: Receiver<NtUpdate>,
    generic_publisher: GenericPublisher,
) {
    loop {
        match receiver.recv().await {
            Ok(msg) => if let NtUpdate::Publish(k, v) = msg {
                let r = generic_publisher.set(k.clone(), v).await;
                match r {
                    Ok(_) => info!("Set key: {}", k),
                    Err(err) => warn!("Error setting key: {}", err),
                }
            },
            Err(e) => {
                error!("error in publish: {e}")
            }
        }
    }
}
