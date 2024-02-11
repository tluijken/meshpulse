use crate::prelude::Subscription;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Mutex;
use lazy_static::lazy_static;
use crate::get_env_var;

#[cfg(feature = "mqtt")]
pub struct MqttSubscription {
    pub topic: String,
    pub id: uuid::Uuid
}

#[cfg(feature = "mqtt")]
impl Subscription for MqttSubscription {
    fn unsubscribe(self) -> Result<(), Box<dyn Error>> {
        let mut mqtt_client = MQTTCLIENT.lock().unwrap();
        let topic = mqtt_client.topics.get_mut(&self.topic);
        match topic {
            Some(topic) => {
                topic.remove(&self.id);
                match topic.len() {
                    0 => {
                        mqtt_client.topics.remove(&self.topic);
                        mqtt_client.client.unsubscribe(&self.topic).unwrap();
                    }
                    _ => {}
                }
            }
            None => {}
        }
        Ok(())
    }
}

#[cfg(feature = "mqtt")]
pub struct MQTTClient {
    pub client: paho_mqtt::Client,
    _thread: std::thread::JoinHandle<()>,
    // create a hashmap of topics with a hashmap of callbacks
    pub topics: HashMap<String, HashMap<uuid::Uuid, Box<dyn FnMut(String) -> () + Send + 'static>>>
}

#[cfg(feature = "mqtt")]
impl MQTTClient {
    pub fn new() -> Self {
        let options = paho_mqtt::ConnectOptionsBuilder::new()
            .user_name(get_env_var("MQTT_USERNAME"))
            .password(get_env_var("MQTT_PASSWORD"))
            .finalize();
        let opt_opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(get_env_var("MQTT_HOST"))
            .finalize();
        let client = paho_mqtt::Client::new(opt_opts).unwrap();
        client.connect(options).expect("Failed to connect");
        let rx = client.start_consuming();
        let thread = std::thread::spawn(move || {
            for msg in rx.iter() {
                match msg {
                    Some(msg) => {
                        let payload = msg.payload_str().to_string();
                        let topic = msg.topic().to_string();
                        match MQTTCLIENT.lock().unwrap().topics.get_mut(&topic) {
                            Some(callbacks) => {
                                for callback in callbacks.values_mut() {
                                    callback(payload.clone());
                                }
                            }
                            None => {}
                        }

                    }
                    None => {}
                }
            }
        });
        Self {
            client,
            _thread: thread,
            topics: HashMap::new(),
        }
    }
}

#[cfg(feature = "mqtt")]
lazy_static! {
    pub static ref MQTTCLIENT: Mutex<MQTTClient> = Mutex::new(MQTTClient::new());
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::prelude::*;

    fn setup_enviroment_variables() {
        std::env::set_var("MQTT_USERNAME", "test");
        std::env::set_var("MQTT_PASSWORD", "test");
        std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
    }

    #[derive(Serialize, Deserialize, Event )]
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
        assert_eq!(MQTTCLIENT.lock().unwrap().topics.len(), 1);

        // Simulate event publication
        TestEvent {
            message: "World".to_string(),
        }
        .publish()
        .expect("Event publication failed");

        // wait for the event to be received
        std::thread::sleep(std::time::Duration::from_millis(10));
        // Cleanup: Unsubscribe if necessary and perform assertions
        let unsub_result = sub_result.unwrap().unsubscribe();
        assert!(unsub_result.is_ok(), "Unsubscription should succeed");

        // Assertion: Verify that the event was received
        let received_count = *received.lock().unwrap();
        assert_eq!(received_count, 1, "Expected exactly 1 event to be received");

        // we should have no topics left
        assert_eq!(MQTTCLIENT.lock().unwrap().topics.len(), 0);
    }
}
