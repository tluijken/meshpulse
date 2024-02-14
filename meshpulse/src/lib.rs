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
    fn publish(&self) -> Result<(), Box<dyn std::error::Error>>;
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
    type Response;
    fn request(
        &self,
    ) -> impl std::future::Future<Output = Result<Self::Response, Box<dyn std::error::Error>>> + Send;
}

/// This trait is used to handle rpc requests using meshpulse
pub trait RpcRequestHandler<R: RpcRequest> {
    fn start(&mut self);
    fn handle_request(request: R) -> Result<R::Response, Box<dyn std::error::Error>>;
    fn stop(&self);
}

// prelude
#[cfg(feature = "mqtt")]
pub mod prelude {
    pub use super::clients::mqtt::MqttSubscription;
    pub use super::clients::mqtt::QOS;
    pub use super::Publish;
    pub use super::RpcRequest;
    pub use super::RpcRequestHandler;
    pub use super::Subscribe;
    pub use super::Subscription;

    // re-exports
    pub use crate::clients::mqtt::MQTTCLIENT;
    pub use meshpulse_derive::Event;
    pub use paho_mqtt;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json;
}

fn get_env_var(name: &str) -> String {
    match std::env::var(name) {
        Ok(val) => val,
        Err(e) => panic!("{} is not set: {}", name, e),
    }
}
