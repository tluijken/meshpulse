use amqprs::channel::{self, Channel};

use crate::prelude::Subscription;

#[cfg(feature = "amqp")]
pub struct AmqpSubscription {
    pub queue: String,
    pub id: uuid::Uuid,
    pub channel: Channel,
}

#[cfg(feature = "amqp")]
impl Subscription for AmqpSubscription {
    fn unsubscribe(self) -> Result<(), Box<dyn std::error::Error>> {
        let amqp_client = super::AMQPCLIENT.read().unwrap();
        let queue = amqp_client.queues.get(&self.queue);
        if queue.is_none() {
            println!("No queue found for: {}", self.queue);
            return Ok(());
        }

        let queue = queue.unwrap();
        let mut queue = queue.lock().unwrap();

        if !queue.contains_key(&self.id) {
            println!(
                "No subscription found by id {} for queue {}",
                self.id, self.queue
            );
            return Ok(());
        }

        queue.remove(&self.id);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            self.channel.close().await.expect("Failed to close channel");
            if queue.len() == 0 {
                amqp_client.connection.to_owned().close().await.expect("Failed to close connection");
            }
        });

        Ok(())
    }
}
