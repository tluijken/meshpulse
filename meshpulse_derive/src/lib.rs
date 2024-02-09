extern crate proc_macro2;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};


#[cfg(feature = "mqtt")]
#[proc_macro_derive(Event, attributes(Event))]
pub fn event_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let expanded = quote! {
        fn get_env_var(name: &str) -> String {
            match std::env::var(name) {
                Ok(val) => val,
                Err(e) => panic!("{} is not set: {}", name, e),
            }
        }
        
        /// creates a new mqtt client and connects to the broker
        /// # Example:
        /// ```
        /// let client = getClient();
        /// ```
        fn getClient() -> paho_mqtt::Client {
            let options = paho_mqtt::ConnectOptionsBuilder::new()
                .user_name(get_env_var("MQTT_USERNAME"))
                .password(get_env_var("MQTT_PASSWORD"))
                .finalize();
            let opt_opts = paho_mqtt::CreateOptionsBuilder::new()
                .server_uri(get_env_var("MQTT_HOST"))
                .finalize();
            let client = paho_mqtt::Client::new(opt_opts).unwrap();
            client.connect(options).expect("Failed to connect");
            client

        }

        impl Publish for #struct_name {
            fn publish(&self) -> Result<(), Box<dyn std::error::Error>> {
                // the topic is the name of the struct, use reflection to get it
                let topic = format!("events/{}", std::any::type_name::<Self>());
                let payload = serde_json::to_string(&self).unwrap();

                let msg = paho_mqtt::Message::new(topic, payload , 0);
                getClient().publish(msg).unwrap();
                Ok(())
            }
        }

        impl Subscribe for #struct_name {
            type Event = Self;
            fn subscribe(mut callback: impl FnMut(Self) -> () + Send + 'static) -> Result<impl Subscription, Box<dyn std::error::Error>> {
                let cli = getClient();
                // the topic is the name of the struct, use reflection to get it
                let topic = format!("events/{}", std::any::type_name::<Self>());
                // loop and call the callback
                let rx = cli.start_consuming();
                cli.subscribe(&topic, 0).unwrap();
                let thread = std::thread::spawn(move || {
                    for msg in rx.iter() {
                        match msg {
                            Some(msg) => {
                                let payload = msg.payload_str().to_string();
                                let event: TestEvent = serde_json::from_str(&payload).unwrap();
                                callback(event);
                            }
                            None => {}
                        }
                    }
                });
                let sub = MqttSubscription {
                    _thread: thread,
                    connection: cli,
                    topic,
                };

                Ok(sub)
            }
        }
    };
    TokenStream::from(expanded)
}
