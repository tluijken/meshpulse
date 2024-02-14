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
        impl Publish for #struct_name {
            fn publish(&self) -> Result<(), Box<dyn std::error::Error>> {
                // the topic is the name of the struct, use reflection to get it
                let topic = format!("events/{}", std::any::type_name::<Self>());
                let payload = serde_json::to_string(&self).unwrap();
                let msg = paho_mqtt::MessageBuilder::new()
                    .topic(topic)
                    .payload(payload)
                    .qos(QOS)
                    .finalize();
                let cli = &MQTTCLIENT.read().unwrap().client;
                cli.publish(msg).unwrap();
                Ok(())
            }
        }

        impl Subscribe for #struct_name {
            type Event = Self;
            fn subscribe(mut callback: impl FnMut(Self) -> () + Send + 'static) -> Result<impl Subscription, Box<dyn std::error::Error>> {
                let mut mqtt_client = MQTTCLIENT.write().unwrap();
                let topic = format!("events/{}", std::any::type_name::<Self>());
                mqtt_client.client.subscribe(&topic, QOS).unwrap();
                let sub = MqttSubscription {
                    topic: topic.clone(),
                    id: uuid::Uuid::new_v4()
                };
                let topic = mqtt_client.topics.entry(sub.topic.clone()).or_insert(std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())));
                let mut topic = topic.lock().unwrap();
                topic.insert(sub.id, Box::new(move |msg: paho_mqtt::Message| {
                    let payload = msg.payload_str().to_string();
                    let event: Self = serde_json::from_str(&payload).unwrap();
                    callback(event);
                }));

                Ok(sub)
            }
        }
    };
    TokenStream::from(expanded)
}
