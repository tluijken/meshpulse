use std::{collections::HashMap, sync::{Arc, Mutex}};

use amqprs::{callbacks::{DefaultChannelCallback, DefaultConnectionCallback}, channel::{BasicAckArguments, BasicPublishArguments, Channel}, connection::{Connection, OpenConnectionArguments}, consumer::AsyncConsumer, BasicProperties, Deliver};
use async_trait;
use serde::{Deserialize, Serialize};
use crate::{get_env_var, Publish};

use super::AMQPCLIENT;

#[cfg(feature = "amqp")]
pub struct AMQPClient {
    connection: Connection,
    channel: Channel,
    pub topics: HashMap<
        String,
        Arc<Mutex<HashMap<uuid::Uuid, Box<dyn FnMut(dyn AsyncConsumer) -> () + Send + 'static>>>>,
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
            topics: HashMap::new(),
        }
    }

    pub async fn disconnect(self) {
        self.channel.close().await.expect("Failed to close channel");
        self.connection.close().await.expect("Failed to close connection");
    }
}

#[derive(Serialize, Deserialize)]
struct TestEvent {
    message: String,
}

impl Publish for TestEvent {
    async fn publish(&self) -> Result<(), Box<dyn std::error::Error>> {
        // the topic is the name of the struct, use reflection to get it
        let exchange_name = format!("events.{}", std::any::type_name::<Self>());
        let routing_key = "meshpulse.events";
        let payload = serde_json::to_string(&self).unwrap();
        let channel = &AMQPCLIENT.read().unwrap().channel;

        // create arguments for basic_publish
        let args = BasicPublishArguments::new(&exchange_name, routing_key);

        channel
            .basic_publish(BasicProperties::default(), payload.into(), args)
            .await
            .unwrap();
        Ok(())
    }
}

struct AMQPConsumer;

#[async_trait::async_trait]
impl AsyncConsumer for AMQPConsumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {

        let string = String::from_utf8(content).expect("Our bytes should be valid utf8");
        println!("Received message: {}", string);
           channel
                .basic_ack(BasicAckArguments::new(deliver.delivery_tag(), true))
                .await
                .unwrap();
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
