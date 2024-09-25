use std::{borrow::BorrowMut, collections::HashMap, sync::{Arc, Mutex}};

use amqprs::{callbacks::{DefaultChannelCallback, DefaultConnectionCallback}, channel::{BasicAckArguments, BasicConsumeArguments, BasicPublishArguments, Channel, QueueBindArguments, QueueDeclareArguments}, connection::{Connection, OpenConnectionArguments}, consumer::AsyncConsumer, BasicProperties, Deliver};
use async_trait;
use serde::{Deserialize, Serialize};
use crate::{get_env_var, Publish, Subscribe, Subscription};

use super::{subscription::AmqpSubscription, AMQPCLIENT};

#[cfg(feature = "amqp")]
pub struct AMQPClient {
    pub connection: Connection,
    channel: Channel,
    pub queues: HashMap<
        String,
        Arc<Mutex<HashMap<uuid::Uuid, Box<dyn FnMut(&str) -> () + Send + 'static>>>>,
    >,
}

#[cfg(feature = "amqp")]
impl AMQPClient {
    pub async fn new() -> Self {
        let username = get_env_var("AMQP_USERNAME");
        let password = get_env_var("AMQP_PASSWORD");
        let host = get_env_var("AMQP_HOST");
        let port = get_env_var("AMQP_PORT");
        let connecttion_options = OpenConnectionArguments::new(
                &host,
                port.parse::<u16>().unwrap(),
                &username,
                &password,
            );

        let connection = Connection::open(&connecttion_options).await.expect("Failed to connect to AMQP server");
        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .unwrap();


        // open a channel on the connection
        let channel = connection.open_channel(None).await.unwrap();
        channel
            .register_callback(DefaultChannelCallback)
            .await
            .unwrap();

        Self {
            channel,
            connection,
            queues: HashMap::new(),
        }
    }

    pub async fn disconnect(self) {
        self.channel.close().await.expect("Failed to close channel");
        self.connection.close().await.expect("Failed to close connection");
    }
}

#[derive(Serialize, Deserialize)]
pub struct TestEvent {
    pub message: String,
}

// Implement the `Publish` trait for `TestEvent`
#[async_trait::async_trait]
impl Publish for TestEvent {
    async fn publish(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let exchange_name = format!("events.{}", std::any::type_name::<Self>());
        let routing_key = "meshpulse.events";
        let payload = serde_json::to_string(&self).unwrap();
        let args = BasicPublishArguments::new(&exchange_name, routing_key);

        // Clone the channel outside of the lock scope to avoid holding the lock during `.await`
        let channel = {
            let client_guard = AMQPCLIENT.read().unwrap(); // Acquire the read lock
            client_guard.channel.clone()  // Clone the channel (assuming `channel` implements `Clone`)
        };

        // Perform the async `basic_publish` operation
        channel
            .basic_publish(BasicProperties::default(), payload.into(), args)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(())
    }
}

impl Subscribe for TestEvent {
    type Event = Self;
    fn subscribe(mut callback: impl FnMut(Self) -> () + Send + 'static) -> Result<impl Subscription, Box<dyn std::error::Error>> {

        let exchange_name = format!("events.{}", std::any::type_name::<Self>());
        let routing_key = format!("events/{}", std::any::type_name::<Self>());

        // First, handle the immutable borrow for accessing the channel
        let queue_name = {
            let amqp_client = AMQPCLIENT.read().unwrap(); // Immutable borrow
            let channel = &amqp_client.channel;

            // Declare a server-named transient queue and bind it
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (queue_name, _, _) = channel
                    .queue_declare(QueueDeclareArguments::default())
                    .await
                    .unwrap()
                    .unwrap();

                channel
                    .queue_bind(QueueBindArguments::new(
                            &queue_name,
                            &exchange_name,
                            routing_key.as_str(),
                            ))
                    .await
                    .unwrap();

                // Start consumer, auto-ack
                let args = BasicConsumeArguments::new(&queue_name, "basic_consumer")
                    .manual_ack(false)
                    .finish();

                channel
                    .basic_consume(AMQPConsumer {}, args)
                    .await
                    .unwrap();

                queue_name // Return queue_name from the block
            })
        };

        let mut amqp_client = AMQPCLIENT.write().unwrap(); // Mutable borrow
        let subscription = AmqpSubscription {
            queue: queue_name,
            channel: amqp_client.channel.clone(),
            id: uuid::Uuid::new_v4(),
        };

        let topic = amqp_client.queues
            .entry(subscription.queue.clone())
            .or_insert_with(|| std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())));

        let mut topic = topic.lock().unwrap();
        topic.insert(subscription.id, Box::new(move |msg: &str| {
            let event: Self = serde_json::from_str(msg).unwrap();
            callback(event);
        }));

        Ok(subscription)
    }
}

struct AMQPConsumer;

#[async_trait::async_trait]
impl AsyncConsumer for AMQPConsumer {
    async fn consume(
        &mut self,
        _channel: &Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {

        let string = String::from_utf8(content).expect("Our bytes should be valid utf8");
        let queue = deliver.routing_key();
        let amqp_client = AMQPCLIENT.read().unwrap();
        let topic = amqp_client.queues.get(queue);
        match topic {
            Some(topic) => {
                let mut topic = topic.lock().unwrap();
                for (_, callback) in topic.iter_mut() {
                    callback(string.as_str());
                }
            }
            None => {
                println!("No topic found for: {}", queue);
            }
        }
    }
}

mod tests {
    use amqprs::{callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicConsumeArguments, BasicPublishArguments, QueueBindArguments,
    QueueDeclareArguments}, consumer::DefaultConsumer, BasicProperties};



    use super::*;

    #[tokio::test]
    async fn test_connect_to_amqp() {
        std::env::set_var("AMQP_USERNAME", "test");
        std::env::set_var("AMQP_PASSWORD", "test");
        std::env::set_var("AMQP_HOST", "localhost");
        std::env::set_var("AMQP_PORT", "5672");
        let _amqp_client = AMQPClient::new().await;
        //amqp_client.client.send("hello world").await.expect("Failed to send message");
    }

    #[tokio::test]
    async fn test_client()
    {
        // open a connection to RabbitMQ server
        let connection = Connection::open(&OpenConnectionArguments::new(
                "localhost",
                5672,
                "test",
                "test",
                ))
            .await
            .unwrap();
        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .unwrap();

        // open a channel on the connection
        let channel = connection.open_channel(None).await.unwrap();
        channel
            .register_callback(DefaultChannelCallback)
            .await
            .unwrap();

        // declare a server-named transient queue
        let (queue_name, _, _) = channel
            .queue_declare(QueueDeclareArguments::default())
            .await
            .unwrap()
            .unwrap();

        // bind the queue to exchange
        let routing_key = "amqprs.example";
        let exchange_name = "amq.topic";
        channel
            .queue_bind(QueueBindArguments::new(
                    &queue_name,
                    exchange_name,
                    routing_key,
                    ))
            .await
            .unwrap();

        //////////////////////////////////////////////////////////////////////////////
        // start consumer, auto ack
        let args = BasicConsumeArguments::new(&queue_name, "basic_consumer")
            .manual_ack(false)
            .finish();

        let consumer = AMQPConsumer {};
        channel
            .basic_consume(consumer, args)
            .await
            .unwrap();

        // publish message
        let content = String::from(
            r#"
            {
                "publisher": "example"
                    "data": "Hello, amqprs!"
            }
            "#,
            )
            .into_bytes();

        // create arguments for basic_publish
        let args = BasicPublishArguments::new(exchange_name, routing_key);

        channel
            .basic_publish(BasicProperties::default(), content, args)
            .await
            .unwrap();

        // keep the `channel` and `connection` object from dropping before pub/sub is done.
        // channel/connection will be closed when drop.
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        // explicitly close

        channel.close().await.unwrap();
        connection.close().await.unwrap();
    }
}
