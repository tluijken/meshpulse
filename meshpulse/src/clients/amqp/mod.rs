pub mod client;
pub mod subscription;
use client::AMQPClient;
use tokio::task;
use once_cell::sync::Lazy;
use std::sync::{Arc, RwLock};

pub static AMQPCLIENT: Lazy<Arc<RwLock<AMQPClient>>> = Lazy::new(|| {
    Arc::new(RwLock::new(task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            AMQPClient::new().await
        })
    })))
});

#[cfg(test)]
mod tests {
    use crate::{clients::amqp::client::TestEvent, prelude::*};

    fn setup_enviroment_variables() {
        std::env::set_var("MQTT_USERNAME", "test");
        std::env::set_var("MQTT_PASSWORD", "test");
        std::env::set_var("AMQP_HOST", "localhost");
        std::env::set_var("AMQP_PORT", "5672");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_publish() {
        setup_enviroment_variables();
        let event = TestEvent {
            message: "hello".to_string(),
        };
        let result = event.publish().await;
        assert_eq!(result.is_ok(), true);
    }
}
