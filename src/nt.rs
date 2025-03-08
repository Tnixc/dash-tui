use nt_client::data::r#type::NetworkTableData;
use nt_client::subscribe::ReceivedMessage;
use nt_client::topic::Topic;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tokio::time;

#[derive(Debug, Clone)]
pub enum NtUpdate {
    KeyValue(String, f64),
}

pub async fn run_nt_client(sender: Sender<NtUpdate>, topic: Topic) {
    let mut counter = 0.0;
    let mut subscriber = topic.subscribe(Default::default()).await;
    loop {
        let k = subscriber.recv();
        match k.await {
            Ok(ReceivedMessage::Announced(topic)) => println!("announced topic: {}", topic.name()),
            Ok(ReceivedMessage::Updated((topic, value))) => {
                let value = format!("{:?}", value);
                let _ = sender.send(NtUpdate::KeyValue("Counter".to_string(), counter));
                // println!("topic {} updated to {value}", topic.name());
            }
            Ok(ReceivedMessage::Unannounced { name, .. }) => {
                // println!("topic {name} unannounced");
            }
            Err(err) => {
                // eprint!("{err:?}");
                break;
            }
        }
        // Simulate Network Tables update with a counter
        counter += 0.001;

        // Send the counter value through the channel
        if sender
            .send(NtUpdate::KeyValue("Counter".to_string(), counter))
            .is_err()
        {
            // Channel is closed, exit the loop
            break;
        }

        // Sleep for 1 second
        time::sleep(Duration::from_millis(1)).await;
    }
}
