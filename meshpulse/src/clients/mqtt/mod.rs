use crate::prelude::Subscription;
pub mod rpc;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex, RwLock};

use crate::get_env_var;
use lazy_static::lazy_static;
use paho_mqtt::Message;

pub const QOS: i32 = 2;

#[cfg(feature = "mqtt")]
pub struct MqttSubscription {
    pub topic: String,
    pub id: uuid::Uuid,
}

#[cfg(feature = "mqtt")]
impl Subscription for MqttSubscription {
    fn unsubscribe(self) -> Result<(), Box<dyn Error>> {
        let mqtt_client = MQTTCLIENT.read().unwrap();
        let topic = mqtt_client.topics.get(&self.topic);
        if topic.is_none() {
            println!("No topic found for: {}", self.topic);
            return Ok(());
        }

        let topic = topic.unwrap();
        let mut topic = topic.lock().unwrap();

        if !topic.contains_key(&self.id) {
            println!(
                "No subscription found by id {} for topic {}",
                self.id, self.topic
            );
            return Ok(());
        }

        topic.remove(&self.id);

        if topic.len() == 0 {
            mqtt_client.client.unsubscribe(&self.topic).unwrap();
        }

        Ok(())
    }
}

#[cfg(feature = "mqtt")]
pub struct MQTTClient {
    pub client: paho_mqtt::Client,
    _thread: std::thread::JoinHandle<()>,
    // create a hashmap of topics with a hashmap of callbacks
    pub topics: HashMap<
        String,
        Arc<Mutex<HashMap<uuid::Uuid, Box<dyn FnMut(Message) -> () + Send + 'static>>>>,
    >,
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
            .mqtt_version(paho_mqtt::MQTT_VERSION_5)
            .finalize();
        let client = paho_mqtt::Client::new(opt_opts).unwrap();
        client.connect(options).expect("Failed to connect");
        let rx = client.start_consuming();
        let thread = std::thread::spawn(move || {
            for msg in rx.iter() {
                match msg {
                    Some(msg) => {
                        let _payload = msg.payload_str().to_string();
                        let topic = msg.topic().to_string();
                        println!("Received message: {}", _payload);

                        // check if we have any callbacks for this topic
                        let client = MQTTCLIENT.read().unwrap();
                        match client.topics.get(&topic) {
                            Some(topic) => {
                                let mut lock = topic.lock().unwrap();
                                for (_, callback) in lock.iter_mut() {
                                    callback(msg.clone());
                                }
                            }
                            None => {
                                println!("No topic found for: {}", topic);
                            }
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
    pub static ref MQTTCLIENT: RwLock<MQTTClient> = RwLock::new(MQTTClient::new());
}

// #[cfg(test)]
// mod tests {
//     use std::sync::{Arc, Mutex};
//
//     use crate::prelude::*;
//
//     const WAIT_DURATION: std::time::Duration = std::time::Duration::from_millis(50);
//     const EVENT_TOPIC: &str = "events/meshpulse::clients::mqtt::tests::TestEvent";
//     const EVENT2_TOPIC: &str = "events/meshpulse::clients::mqtt::tests::TestEvent2";
//
//     fn setup_enviroment_variables() {
//         std::env::set_var("MQTT_USERNAME", "test");
//         std::env::set_var("MQTT_PASSWORD", "test");
//         std::env::set_var("MQTT_HOST", "tcp://localhost:1883");
//     }
//
//     #[derive(Serialize, Deserialize, Event)]
//     struct TestEvent {
//         message: String,
//     }
//
//     #[derive(Serialize, Deserialize, Event)]
//     struct TestEvent2 {
//         message: String,
//     }
//
//     #[test]
//     fn test_publish() {
//         setup_enviroment_variables();
//         let event = TestEvent {
//             message: "hello".to_string(),
//         };
//         let result = event.publish();
//         assert_eq!(result.is_ok(), true);
//     }
//
//     #[test]
//     fn test_subscribe() {
//         setup_enviroment_variables();
//         // Setup: Prepare a shared counter for event reception
//         let received = Arc::new(Mutex::new(0));
//
//         let sub_result = TestEvent::subscribe({
//             let received_clone = Arc::clone(&received);
//             move |event: TestEvent| {
//                 if event.message == "World" {
//                     let mut count = received_clone.lock().unwrap();
//                     *count += 1;
//                 }
//             }
//         });
//
//         assert!(sub_result.is_ok(), "Subscription should succeed");
//         assert_eq!(
//             MQTTCLIENT
//                 .read()
//                 .unwrap()
//                 .topics
//                 .get(EVENT_TOPIC)
//                 .unwrap()
//                 .lock()
//                 .unwrap()
//                 .len(),
//             1,
//             "Expected exactly 1 subscription to be present"
//         );
//
//         TestEvent {
//             message: "World".to_string(),
//         }
//         .publish()
//         .expect("Event publication failed");
//
//         // wait for the event to be received
//         std::thread::sleep(WAIT_DURATION);
//
//         // Cleanup: Unsubscribe if necessary and perform assertions
//         let unsub_result = sub_result.unwrap().unsubscribe();
//         assert!(unsub_result.is_ok(), "Unsubscription should succeed");
//
//         let received_count = *received.lock().unwrap();
//         assert_eq!(received_count, 1, "Expected exactly 1 event to be received");
//     }
//
//     #[test]
//     fn test_double_subscribe() {
//         setup_enviroment_variables();
//
//         let received_1 = Arc::new(Mutex::new(0));
//         let received_2 = Arc::new(Mutex::new(0));
//         let sub_result_1 = TestEvent2::subscribe({
//             let received_clone = Arc::clone(&received_1);
//             move |event: TestEvent2| {
//                 if event.message == "Double" {
//                     let mut count = received_clone.lock().unwrap();
//                     *count += 1;
//                 }
//             }
//         });
//         assert_eq!(sub_result_1.is_ok(), true);
//         let sub_result_2 = TestEvent2::subscribe({
//             let received_clone = Arc::clone(&received_2);
//             move |event: TestEvent2| {
//                 if event.message == "Double" {
//                     let mut count = received_clone.lock().unwrap();
//                     *count += 1;
//                     println!("Received event: {:?}", event.message);
//                 }
//             }
//         });
//         assert_eq!(sub_result_1.is_ok(), true);
//         assert_eq!(sub_result_2.is_ok(), true);
//         assert_eq!(
//             MQTTCLIENT
//                 .read()
//                 .unwrap()
//                 .topics
//                 .get(EVENT2_TOPIC)
//                 .unwrap()
//                 .lock()
//                 .unwrap()
//                 .len(),
//             2,
//             "Expected exactly 2 subscriptions to be present"
//         );
//         TestEvent2 {
//             message: "Double".to_string(),
//         }
//         .publish()
//         .expect("Event publication failed");
//         //
//         std::thread::sleep(WAIT_DURATION);
//         let received_count_1 = *received_1.lock().unwrap();
//         let received_count_2 = *received_2.lock().unwrap();
//         assert_eq!(
//             received_count_1, 1,
//             "Expected exactly 1 event to be received"
//         );
//         assert_eq!(
//             received_count_2, 1,
//             "Expected exactly 1 event to be received"
//         );
//         // unsubscribe the first callback
//         let unsub_result = sub_result_1.unwrap().unsubscribe();
//         assert_eq!(unsub_result.is_ok(), true);
//         // check if one callback was removed
//         assert_eq!(
//             MQTTCLIENT
//                 .read()
//                 .unwrap()
//                 .topics
//                 .get(EVENT2_TOPIC)
//                 .unwrap()
//                 .lock()
//                 .unwrap()
//                 .len(),
//             1,
//             "Expected exactly 1 subscription to be present"
//         );
//
//         // publish the event again
//         TestEvent2 {
//             message: "Double".to_string(),
//         }
//         .publish()
//         .expect("Event publication failed");
//
//         std::thread::sleep(WAIT_DURATION);
//         let received_count_1 = *received_1.lock().unwrap();
//         let received_count_2 = *received_2.lock().unwrap();
//
//         // check if the first callback didn't react to the event
//         assert_eq!(
//             received_count_1, 1,
//             "Expected exactly 1 event to be received"
//         );
//
//         // check if the second callback reacted to the event
//         assert_eq!(
//             received_count_2, 2,
//             "Expected exactly 2 event to be received"
//         );
//
//         // unsubscribe the second callback
//         let unsub_result = sub_result_2.unwrap().unsubscribe();
//         assert_eq!(unsub_result.is_ok(), true);
//         // check if the last callback was removed
//         assert_eq!(
//             MQTTCLIENT
//                 .read()
//                 .unwrap()
//                 .topics
//                 .get(EVENT2_TOPIC)
//                 .unwrap()
//                 .lock()
//                 .unwrap()
//                 .len(),
//             0,
//             "Expected exactly 0 subscriptions to be present"
//         );
//     }
// }
