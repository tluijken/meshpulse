pub mod client;
pub mod rpc;
pub mod subscription;
use lazy_static::lazy_static;
pub use rpc::RpcRequestHandler;
use std::sync::RwLock;

pub const QOS: i32 = 2;

lazy_static! {
    pub static ref MQTTCLIENT: RwLock<client::MQTTClient> = RwLock::new(client::MQTTClient::new());
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::prelude::*;

    const WAIT_DURATION: std::time::Duration = std::time::Duration::from_millis(50);
    const EVENT_TOPIC: &str = "events/meshpulse::clients::mqtt::tests::TestEvent";
    const EVENT2_TOPIC: &str = "events/meshpulse::clients::mqtt::tests::TestEvent2";

    fn setup_enviroment_variables() {
        std::env::set_var("MQTT_USERNAME", "test");
        std::env::set_var("MQTT_PASSWORD", "test");
        std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
    }

    #[derive(Serialize, Deserialize, Event)]
    struct TestEvent {
        message: String,
    }

    #[derive(Serialize, Deserialize, Event)]
    struct TestEvent2 {
        message: String,
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_subscribe() {
        setup_enviroment_variables();
        // Setup: Prepare a shared counter for event reception
        let received = Arc::new(Mutex::new(0));

        let sub_result = TestEvent::subscribe({
            let received_clone = Arc::clone(&received);
            move |event: TestEvent| {
                if event.message == "World" {
                    let mut count = received_clone.lock().unwrap();
                    *count += 1;
                }
            }
        });

        assert!(sub_result.is_ok(), "Subscription should succeed");
        assert_eq!(
            MQTTCLIENT
                .read()
                .unwrap()
                .topics
                .get(EVENT_TOPIC)
                .unwrap()
                .lock()
                .unwrap()
                .len(),
            1,
            "Expected exactly 1 subscription to be present"
        );

        TestEvent {
            message: "World".to_string(),
        }
        .publish()
        .await
        .expect("Event publication failed");

        // wait for the event to be received
        std::thread::sleep(WAIT_DURATION);

        // Cleanup: Unsubscribe if necessary and perform assertions
        let unsub_result = sub_result.unwrap().unsubscribe();
        assert!(unsub_result.is_ok(), "Unsubscription should succeed");

        let received_count = *received.lock().unwrap();
        assert_eq!(received_count, 1, "Expected exactly 1 event to be received");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_double_subscribe() {
        setup_enviroment_variables();

        let received_1 = Arc::new(Mutex::new(0));
        let received_2 = Arc::new(Mutex::new(0));
        let sub_result_1 = TestEvent2::subscribe({
            let received_clone = Arc::clone(&received_1);
            move |event: TestEvent2| {
                if event.message == "Double" {
                    let mut count = received_clone.lock().unwrap();
                    *count += 1;
                }
            }
        });
        assert_eq!(sub_result_1.is_ok(), true);
        let sub_result_2 = TestEvent2::subscribe({
            let received_clone = Arc::clone(&received_2);
            move |event: TestEvent2| {
                if event.message == "Double" {
                    let mut count = received_clone.lock().unwrap();
                    *count += 1;
                    println!("Received event: {:?}", event.message);
                }
            }
        });
        assert_eq!(sub_result_1.is_ok(), true);
        assert_eq!(sub_result_2.is_ok(), true);
        assert_eq!(
            MQTTCLIENT
                .read()
                .unwrap()
                .topics
                .get(EVENT2_TOPIC)
                .unwrap()
                .lock()
                .unwrap()
                .len(),
            2,
            "Expected exactly 2 subscriptions to be present"
        );
        TestEvent2 {
            message: "Double".to_string(),
        }
        .publish()
        .await
        .expect("Event publication failed");
        //
        std::thread::sleep(WAIT_DURATION);
        let received_count_1 = *received_1.lock().unwrap();
        let received_count_2 = *received_2.lock().unwrap();
        assert_eq!(
            received_count_1, 1,
            "Expected exactly 1 event to be received"
        );
        assert_eq!(
            received_count_2, 1,
            "Expected exactly 1 event to be received"
        );
        // unsubscribe the first callback
        let unsub_result = sub_result_1.unwrap().unsubscribe();
        assert_eq!(unsub_result.is_ok(), true);
        // check if one callback was removed
        assert_eq!(
            MQTTCLIENT
                .read()
                .unwrap()
                .topics
                .get(EVENT2_TOPIC)
                .unwrap()
                .lock()
                .unwrap()
                .len(),
            1,
            "Expected exactly 1 subscription to be present"
        );

        // publish the event again
        TestEvent2 {
            message: "Double".to_string(),
        }
        .publish()
        .await
        .expect("Event publication failed");

        std::thread::sleep(WAIT_DURATION);
        let received_count_1 = *received_1.lock().unwrap();
        let received_count_2 = *received_2.lock().unwrap();

        // check if the first callback didn't react to the event
        assert_eq!(
            received_count_1, 1,
            "Expected exactly 1 event to be received"
        );

        // check if the second callback reacted to the event
        assert_eq!(
            received_count_2, 2,
            "Expected exactly 2 event to be received"
        );

        // unsubscribe the second callback
        let unsub_result = sub_result_2.unwrap().unsubscribe();
        assert_eq!(unsub_result.is_ok(), true);
        // check if the last callback was removed
        assert_eq!(
            MQTTCLIENT
                .read()
                .unwrap()
                .topics
                .get(EVENT2_TOPIC)
                .unwrap()
                .lock()
                .unwrap()
                .len(),
            0,
            "Expected exactly 0 subscriptions to be present"
        );
    }
}
