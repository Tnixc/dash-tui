use nt_client::subscribe::ReceivedMessage;
use nt_client::topic::Topic;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum NtUpdate {
    KeyValue(String, String),
}

pub async fn run_nt_client(sender: Sender<NtUpdate>, topic: Topic) {
    let mut subscriber = topic.subscribe(Default::default()).await;
    loop {
        let k = subscriber.recv();
        match k.await {
            Ok(ReceivedMessage::Announced(topic)) => {}
            Ok(ReceivedMessage::Updated((topic, value))) => {
                let value = value.to_string().trim().to_string();
                let _ = sender.send(NtUpdate::KeyValue("Akit timestamp".to_string(), value));
            }
            Ok(ReceivedMessage::Unannounced { name, .. }) => {}
            Err(err) => {
                eprint!("{err:?}");
                break;
            }
        }
    }
}
