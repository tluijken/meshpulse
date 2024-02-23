use crate::prelude::Subscription;

pub struct MqttSubscription {
    pub topic: String,
    pub id: uuid::Uuid,
}

impl Subscription for MqttSubscription {
    fn unsubscribe(self) -> Result<(), Box<dyn std::error::Error>> {
        let mqtt_client = super::MQTTCLIENT.read().unwrap();
        let topic = mqtt_client.topics.get(&self.topic);
        if topic.is_none() {
            println!("No topic found for: {}", self.topic);
            return Ok(());
        }

        let topic = topic.unwrap();
        let mut topic = topic.lock().unwrap();

        if !topic.contains_key(&self.id) {
            println!(
                "No subscription found by id {} for topic {}",
                self.id, self.topic
            );
            return Ok(());
        }

        topic.remove(&self.id);

        if topic.len() == 0 {
            mqtt_client.client.unsubscribe(&self.topic).unwrap();
        }

        Ok(())
    }
}
