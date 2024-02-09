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
/// let sub_result = TestEvent::subscribe({
///     move |event: TestEvent| {
///         println!("Received event: {:?}", event.message);
///     }
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

// prelude
#[cfg(feature = "mqtt")]
pub mod prelude {
    pub use super::clients::mqtt::MqttSubscription;
    pub use super::Publish;
    pub use super::Subscribe;
    pub use super::Subscription;

    // re-exports
    pub use meshpulse_derive::Event;
    pub use paho_mqtt;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json;
}
