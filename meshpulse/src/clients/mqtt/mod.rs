use std::error::Error;

// create a generic subscription struct for type T which can hold the subscription thread alive
pub struct Subscription {
    _thread: std::thread::JoinHandle<()>,
    topic: String,
    connection: paho_mqtt::Client,
}

impl Subscription {
    pub fn unsubscribe(self) -> Result<(), Box<dyn Error>> {
        self.connection.unsubscribe(&self.topic).unwrap();
        self.connection.disconnect(None).unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    // create a client that actually connects to a broker
    use super::*;
    use serde::{Serialize, Deserialize};
    use meshpulse_derive::Event;
    use crate::prelude::*;

    fn setup_enviroment_variables() {
        std::env::set_var("MQTT_USERNAME", "test");
        std::env::set_var("MQTT_PASSWORD", "test");
        std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
    }

    #[derive(Serialize,Deserialize, Event)]
    struct TestEvent {
        message: String,
    }

    #[test]
    fn test_publish() {
        setup_enviroment_variables();
        let event = TestEvent {
            message: "hello".to_string(),
        };
        let result = event.publish();
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_subscribe() {
        setup_enviroment_variables();
        // Setup: Prepare a shared counter for event reception
        let received = Arc::new(Mutex::new(0));

        // Action: Subscribe to TestEvent and publish an event
        let sub_result = TestEvent::subscribe({
            let received_clone = Arc::clone(&received);
            move |event: TestEvent| {
                if event.message == "World" {
                    let mut count = received_clone.lock().unwrap();
                    *count += 1;
                    println!("Received event: {:?}", event.message);
                }
            }
        });

        assert!(sub_result.is_ok(), "Subscription should succeed");

        // Simulate event publication
        TestEvent { message: "World".to_string() }.publish().expect("Event publication failed");

        // Cleanup: Unsubscribe if necessary and perform assertions
        let unsub_result = sub_result.unwrap().unsubscribe();
        assert!(unsub_result.is_ok(), "Unsubscription should succeed");

        // Assertion: Verify that the event was received
        let received_count = *received.lock().unwrap();
        assert_eq!(received_count, 1, "Expected exactly 1 event to be received");
    }
}
