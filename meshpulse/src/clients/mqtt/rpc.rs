use crate::prelude::*;

#[derive(Serialize, Deserialize, RpcRequest)]
struct TestRpcRequest {
    message: String,
}

struct Handler {
    subscription: MqttSubscription,
}

impl Handler {
    // start accepts a function that takes a TRequest and returns something that can be serialized
    fn start<TRequest, TResponse>(
        handle_fn: fn(TRequest) -> Result<TResponse, Box<dyn std::error::Error>>,
    ) -> Self
    where
        TRequest: RpcRequest + 'static + serde::de::DeserializeOwned,
        TResponse: serde::Serialize + 'static,
    {
        let request_topic = format!("rpc/+/{}", std::any::type_name::<TRequest>());
        let shared_topic = format!("$share/meshpulse/{}", request_topic);
        let subscription = MqttSubscription {
            topic: request_topic.clone(),
            id: uuid::Uuid::new_v4(),
        };

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
            subscription.id,
            Box::new(move |msg: paho_mqtt::Message| {
                let payload = msg.payload_str().to_string();
                let request: TRequest = serde_json::from_str(&payload).unwrap();
                let response = handle_fn(request).unwrap();
                let response = serde_json::to_string(&RpcResponse { response }).unwrap();
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
        Self { subscription }
    }

    fn stop(&self) {
        let mut mqtt_client = MQTTCLIENT.write().unwrap();
        mqtt_client
            .client
            .unsubscribe(&self.subscription.topic)
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

    fn handle_request(_request: TestRpcRequest) -> Result<String, Box<dyn std::error::Error>> {
        Ok("World".to_string())
    }
    #[tokio::test]
    async fn test_rpc() {
        setup_enviroment_variables();

        let handler = Handler::start(handle_request);

        let request = TestRpcRequest {
            message: "Hello".to_string(),
        };
        let response = request.request::<String>().await.unwrap();
        assert_eq!(response, "World");
        handler.stop();
    }
}
