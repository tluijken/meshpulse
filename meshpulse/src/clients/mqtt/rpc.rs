use crate::prelude::*;
use meshpulse_derive::RpcRequest;
use paho_mqtt::Message;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Serialize, Deserialize, RpcRequest)]
struct TestRpcRequest {
    message: String,
}

struct TestRpcRequestHandler {
    subscription: Option<MqttSubscription>,
}

impl RpcRequestHandler<TestRpcRequest> for TestRpcRequestHandler {
     fn start(&mut self) {
        let request_topic = format!(
            "rpc/+/{}",
            std::any::type_name::<TestRpcRequest>()
        );
        let shared_topic = format!("$share/meshpulse/{}", request_topic);
        self.subscription = Some(MqttSubscription {
            topic: request_topic.clone(),
            id: uuid::Uuid::new_v4(),
        });

        let mut mqtt_client = MQTTCLIENT.write().unwrap();
        mqtt_client.client.subscribe(&shared_topic, QOS).unwrap();
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
                let response_topic = format!("{}/response", msg.topic());
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
    fn handle_request(_request: TestRpcRequest) -> Result<String, Box<dyn std::error::Error>> {
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
