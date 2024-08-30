/// This crate is used to publish and subscribe to events using meshpulse
/// Meshpulse offerts a couple of clients to publish and subscribe to events such as mqtt, amqp, gRPC and http
/// Adding meshpulse to your project is as simple as adding the following to your Cargo.toml
/// ```toml
/// [dependencies]
/// meshpulse = { version = "0.1.0", features = ["mqtt"] }
/// ```
pub mod clients;

/// This trait is used to publish events using meshpulse
/// # Example:
/// ```
/// use meshpulse::prelude::*;
///
/// std::env::set_var("MQTT_USERNAME", "test");
/// std::env::set_var("MQTT_PASSWORD", "test");
/// std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
///
/// #[derive(Serialize, Deserialize, Event)]
/// struct TestEvent {
///    message: String,
/// }
///
/// let event = TestEvent {
///   message: "hello".to_string(),
/// };
/// let result = event.publish();
/// assert_eq!(result.is_ok(), true);
/// ```
pub trait Publish {
    async fn publish(&self) -> Result<(), Box<dyn std::error::Error>>;
}

/// This trait is used to subscribe to events using meshpulse
/// # Example:
/// ```
/// use meshpulse::prelude::*;
///
/// std::env::set_var("MQTT_USERNAME", "test");
/// std::env::set_var("MQTT_PASSWORD", "test");
/// std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
///
/// #[derive(Serialize, Deserialize, Event)]
/// struct TestEvent {
///   message: String,
/// }
///
/// let sub_result = TestEvent::subscribe(|event| {
///         println!("Received event: {:?}", event.message);
///  });
///
///  assert_eq!(sub_result.is_ok(), true);
///
pub trait Subscribe {
    type Event;
    fn subscribe(
        callback: impl FnMut(Self::Event) -> () + Send + 'static,
    ) -> Result<impl Subscription, Box<dyn std::error::Error>>;
}

/// This trait is used to handle rpc requests using meshpulse
pub trait RequestHandler {
    /// Starts a new request handler with the given handle function
    /// The handle function should take a request and return a result
    /// The request type should implement RpcRequest, serde::de::DeserializeOwned and serde::Serialize
    /// The response type should implement serde::Serialize
    /// # Example
    /// ```
    /// use meshpulse::prelude::*;
    /// use serde::{Serialize, Deserialize};
    /// std::env::set_var("MQTT_USERNAME", "test");
    /// std::env::set_var("MQTT_PASSWORD", "test");
    /// std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
    ///
    /// #[derive(Serialize, Deserialize, RpcRequest)]
    /// struct TestRpcRequest(String);
    ///
    /// fn handle_request(request: TestRpcRequest) -> Result<String, Box<dyn std::error::Error>> {
    ///    Ok("World".to_string())
    /// }
    /// let handler = RpcRequestHandler::start(handle_request);
    /// // ... when you no longer need the handler
    /// handler.stop();
    /// ```
    fn start<TRequest, TResponse>(
        handle_fn: fn(TRequest) -> Result<TResponse, Box<dyn std::error::Error>>,
    ) -> Self
    where
        TRequest: RpcRequest + 'static + serde::de::DeserializeOwned + serde::Serialize,
        TResponse: serde::Serialize + 'static;

    /// Stops the request handler
    /// # Example
    /// ```
    /// use meshpulse::prelude::*;
    /// use serde::{Serialize, Deserialize};
    /// std::env::set_var("MQTT_USERNAME", "test");
    /// std::env::set_var("MQTT_PASSWORD", "test");
    /// std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
    ///
    /// #[derive(Serialize, Deserialize, RpcRequest)]
    /// struct MultiplierRequest(i32);
    /// fn handle_request(request: MultiplierRequest) -> Result<i32, Box<dyn std::error::Error>> {
    ///   Ok(request.0 * 2)
    /// }
    /// let handler = RpcRequestHandler::start(handle_request);
    /// // ... when you no longer need the handler
    /// handler.stop();
    /// ```
    fn stop(&self);
}

/// This trait is used to unsubscribe from events using meshpulse
/// # Example:
/// ```
/// use meshpulse::prelude::*;
///
/// std::env::set_var("MQTT_USERNAME", "test");
/// std::env::set_var("MQTT_PASSWORD", "test");
/// std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
///
/// #[derive(Serialize, Deserialize, Event)]
/// struct TestEvent {
///     message: String,
/// }
/// let sub_result = TestEvent::subscribe({
///    move |event: TestEvent| {
///    println!("Received event: {:?}", event.message);
/// }
/// });
/// assert_eq!(sub_result.is_ok(), true);
/// let sub = sub_result.unwrap();
/// let result = sub.unsubscribe();
/// assert_eq!(result.is_ok(), true);
pub trait Subscription {
    fn unsubscribe(self) -> Result<(), Box<dyn std::error::Error>>;
}

/// This trait is used to make rpc requests using meshpulse
pub trait RpcRequest {
    /// Publishes the RpcRequest and returns the response
    /// # Example
    /// ```
    /// use meshpulse::prelude::*;
    /// use serde::{Serialize, Deserialize};
    /// std::env::set_var("MQTT_USERNAME", "test");
    /// std::env::set_var("MQTT_PASSWORD", "test");
    /// std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
    /// #[derive(Serialize, Deserialize, RpcRequest)]
    /// struct TestRpcRequest{
    ///    message: String
    /// }
    /// // execute the request
    /// async fn execute_request() {
    ///     let request = TestRpcRequest {
    ///         message: "hello".to_string(),
    ///     };
    ///     let response = request.request::<String>().await.unwrap();
    /// }
    fn request<TResponse>(
        &self,
    ) -> impl std::future::Future<Output = Result<TResponse, Box<dyn std::error::Error>>> + Send
    where
        TResponse: serde::de::DeserializeOwned + 'static;
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RpcResponse<T> {
    pub response: T,
}

// prelude
#[cfg(feature = "mqtt")]
pub mod prelude {
    pub use super::clients::mqtt::subscription::MqttSubscription;
    pub use super::clients::mqtt::RpcRequestHandler;
    pub use super::clients::mqtt::QOS;
    pub use super::Publish;
    pub use super::RequestHandler;
    pub use super::RpcRequest;
    pub use super::RpcResponse;
    pub use super::Subscribe;
    pub use super::Subscription;

    // re-exports
    pub use crate::clients::mqtt::MQTTCLIENT;
    pub use meshpulse_derive::Event;
    pub use meshpulse_derive::RpcRequest;
    pub use paho_mqtt;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json;
}


fn get_env_var(key: &str) -> String {
    match std::env::var(key) {
        Ok(val) => val,
        Err(_) => panic!("{} not found in environment", key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_env_var_success() {
        std::env::set_var("MQTT_USERNAME", "test");
        std::env::set_var("MQTT_PASSWORD", "test");
        std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
        let mqtt_username = get_env_var("MQTT_USERNAME");
        assert_eq!(mqtt_username, "test");

    }

    #[test]
    fn test_get_env_var_failure() {
        std::env::set_var("MQTT_USERNAME", "test");
        let result = std::panic::catch_unwind(|| get_env_var("NONE_EXISTING_ENV_VAR"));
        assert!(result.is_err());
    }
}
