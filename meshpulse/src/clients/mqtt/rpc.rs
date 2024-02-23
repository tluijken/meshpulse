use crate::prelude::*;

/// This struct is used to handle rpc requests using mqtt
/// It is used to start and stop the rpc request handler
pub struct RpcRequestHandler {
    subscription: MqttSubscription,
}

/// This struct is used to handle rpc requests using mqtt
/// It is used to start and stop the rpc request handler
impl RequestHandler for RpcRequestHandler {
    #[allow(dead_code)]
    fn start<TRequest, TResponse>(
        handle_fn: fn(TRequest) -> Result<TResponse, Box<dyn std::error::Error>>,
    ) -> Self
    where
        TRequest: RpcRequest + 'static + serde::de::DeserializeOwned + serde::Serialize,
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

    #[allow(dead_code)]
    fn stop(&self) {
        let mqtt_client = MQTTCLIENT.write().unwrap();
        mqtt_client
            .client
            .unsubscribe(&self.subscription.topic)
            .unwrap();
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, RpcRequest)]
    struct TestRpcRequest(String);

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
        let handler = RpcRequestHandler::start(handle_request);
        let request = TestRpcRequest("Hello".to_string());
        let response = request.request::<String>().await.unwrap();
        assert_eq!(response, "World");
        handler.stop();
    }

    // test for a request that returns a different type
    #[derive(Serialize, Deserialize, RpcRequest)]
    struct MultiplierRequest {
        number: i32,
        multiplier: i32,
    }

    fn handle_multiplier_request(
        request: MultiplierRequest,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        Ok(request.number * request.multiplier)
    }

    #[tokio::test]
    async fn test_multiplier_rpc() {
        setup_enviroment_variables();
        let handler = RpcRequestHandler::start(handle_multiplier_request);
        let request = MultiplierRequest {
            number: 5,
            multiplier: 2,
        };
        let response = request.request::<i32>().await.unwrap();
        assert_eq!(response, 10);
        handler.stop();
    }
}
