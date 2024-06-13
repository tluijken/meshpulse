use paho_mqtt::Message;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
                        let topic = match topic.starts_with("rpc/") && !topic.ends_with("/response")
                        {
                            true => {
                                let topic = topic.split("/").collect::<Vec<&str>>();
                                format!("rpc/+/{}", topic[2])
                            }
                            false => topic,
                        };

                        // check if we have any callbacks for this topic
                        let client = crate::prelude::MQTTCLIENT.read().unwrap();
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
        let result = std::panic::catch_unwind(|| get_env_var("MQTT_PASSWORD"));
        assert!(result.is_err());
    }
}
