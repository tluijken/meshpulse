use crate::prelude::*;
use paho_mqtt::Message;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::sync::mpsc;

#[derive(Serialize, Deserialize)]
struct TestRpcRequest {
    message: String,
}

impl RpcRequest for TestRpcRequest {
    type Response = String;

    async fn request(&self) -> Result<Self::Response, Box<dyn std::error::Error>> {
        let (tx, mut rx) = mpsc::channel(1);
        // Listen for the response asynchronously
        let topic = format!("rpc/{}", std::any::type_name::<Self>());
        let response_topic = format!("rpc/reply/{}", uuid::Uuid::new_v4());
        let payload = serde_json::to_string(&self).unwrap();

        let mut properties = paho_mqtt::Properties::new();
        properties
            .push_string(paho_mqtt::PropertyCode::ResponseTopic, &response_topic)
            .unwrap();

        let msg = paho_mqtt::MessageBuilder::new()
            .topic(&topic)
            .payload(payload)
            .properties(properties)
            .qos(QOS)
            .finalize();

        tokio::spawn(async move {
            let sub = MqttSubscription {
                topic: response_topic.clone(),
                id: uuid::Uuid::new_v4(),
            };

            {
                let mut mqtt_client = MQTTCLIENT.write().unwrap();
                let topic = &mqtt_client.topics.entry(response_topic.clone()).or_insert(
                    std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
                );

                let mut topic = topic.lock().unwrap();

                // await till we get a response
                topic.insert(
                    sub.id,
                    Box::new(move |msg: Message| {
                        let payload = msg.payload_str().to_string();
                        tx.try_send(payload).unwrap();
                    }),
                );
            }
            {
                let cli = &MQTTCLIENT.read().unwrap().client;
                cli.publish(msg).unwrap();
            }
        });

        // Timeout for response (optional, but recommended)
        println!("Waiting for response");
        let timeout = tokio::time::timeout(std::time::Duration::from_secs(30), rx.recv()).await;
        //todo unsubscribe from response topic
        match timeout {
            Ok(Some(message)) => {
                let response: Self::Response = serde_json::from_str(&message)?;
                Ok(response)
            }
            Ok(None) => Err("No response received".into()),
            Err(_) => Err("Response timeout".into()),
        }
    }
}

struct TestRpcRequestHandler {
    subscription: Option<MqttSubscription>,
}

impl RpcRequestHandler<TestRpcRequest> for TestRpcRequestHandler {
    // keep track of the subscriptions
    fn start(&mut self) {
        let request_topic = format!(
            "$share/meshpulse/rpc/{}",
            std::any::type_name::<TestRpcRequest>()
        );
        assert_eq!(
            request_topic,
            "$share/meshpulse/rpc/meshpulse::clients::mqtt::rpc::TestRpcRequest"
        );

        self.subscription = Some(MqttSubscription {
            topic: request_topic.clone(),
            id: uuid::Uuid::new_v4(),
        });

        println!("Subscribing to request topic: {}", request_topic);

        let mut mqtt_client = MQTTCLIENT.write().unwrap();
        let topic = mqtt_client
            .topics
            .entry(request_topic.clone())
            .or_insert(std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::new(),
            )));
        let mut topic = topic.lock().unwrap();
        topic.insert(
            self.subscription.as_ref().unwrap().id,
            Box::new(move |msg: Message| {
                println!("Received request: {:?}", msg.payload_str());
                let payload = msg.payload_str().to_string();
                let request: TestRpcRequest = serde_json::from_str(&payload).unwrap();
                let response = Self::handle_request(request).unwrap();
                let response_topic = msg
                    .properties()
                    .get_string(paho_mqtt::PropertyCode::ResponseTopic)
                    .unwrap();
                let response_msg = paho_mqtt::MessageBuilder::new()
                    .topic(response_topic)
                    .payload(response)
                    .qos(2)
                    .finalize();
                let cli = &MQTTCLIENT.read().unwrap().client;
                cli.publish(response_msg).unwrap();
            }),
        );
    }

    fn handle_request(_request: TestRpcRequest) -> Result<String, Box<dyn Error>> {
        Ok("World".to_string())
    }

    fn stop(&self) {
        let mqtt_client = MQTTCLIENT.write().unwrap();
        mqtt_client
            .client
            .unsubscribe(&self.subscription.as_ref().unwrap().topic)
            .unwrap();
    }
}

// test
#[cfg(test)]
pub mod tests {
    use super::*;

    fn setup_enviroment_variables() {
        std::env::set_var("MQTT_USERNAME", "test");
        std::env::set_var("MQTT_PASSWORD", "test");
        std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
    }
    #[tokio::test]
    async fn test_rpc() {
        setup_enviroment_variables();

        let mut handler = TestRpcRequestHandler { subscription: None };
        handler.start();
        let request = TestRpcRequest {
            message: "Hello".to_string(),
        };
        let response = request.request().await.unwrap();
        assert_eq!(response, "World");
        handler.stop();
    }
}
