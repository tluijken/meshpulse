use crate::prelude::*;

#[derive(Serialize, Deserialize, RpcRequest)]
struct TestRpcRequest {
    message: String,
}

#[request_handler]
fn handle_request(_request: TestRpcRequest) -> Result<String, Box<dyn std::error::Error>> {
    Ok("World".to_string())
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
        assert_eq!(response, serde_json::to_string(&RpcResponse { response: "World".to_string() }).unwrap());
        handler.stop();
    }
}
