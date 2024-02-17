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

#[cfg(feature = "mqtt")]
#[proc_macro_derive(RpcRequest, attributes(RpcRequest))]
pub fn rpcrequest_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let expanded = quote! {

        impl RpcRequest for #struct_name {
            type Response = String;

            async fn request(&self) -> Result<Self::Response, Box<dyn std::error::Error>> {
                let (tx, mut rx) = tokio::sync::mpsc::channel(1);

                let payload = serde_json::to_string(&self).unwrap();
                let request_topic = format!("rpc/{}/{}", uuid::Uuid::new_v4(), std::any::type_name::<Self>());
                let response_topic = format!("{}/response", request_topic);
                let sub = MqttSubscription {
                    topic: response_topic.clone(),
                    id: uuid::Uuid::new_v4(),
                };
                tokio::spawn(async move {

                    let msg = paho_mqtt::MessageBuilder::new()
                        .topic(&request_topic)
                        .payload(payload)
                        .qos(QOS)
                        .finalize();

                    {
                        let mut mqtt_client = MQTTCLIENT.write().unwrap();
                        mqtt_client.client.subscribe(&response_topic, QOS).unwrap();
                        let topic = &mqtt_client.topics.entry(response_topic.clone()).or_insert(
                            std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
                            );

                        let mut topic = topic.lock().unwrap();

                        // await till we get a response
                        topic.insert(
                            sub.id,
                            Box::new(move |msg: paho_mqtt::Message| {
                                let payload = msg.payload_str().to_string();
                                tx.try_send(payload).unwrap();
                            }),
                            );
                    }
                    {
                        let cli = &MQTTCLIENT.read().unwrap().client;
                        cli.publish(msg).unwrap_or_else(|e| println!("Failed to publish: {}", e));
                    }
                });

                // Timeout for response (optional, but recommended)
                let timeout = tokio::time::timeout(std::time::Duration::from_secs(30), rx.recv()).await;
                sub.unsubscribe().unwrap_or_else(|e| println!("Failed to unsubscribe: {}", e));
                match timeout {
                    Ok(Some(message)) => {

                        let response: Self::Response = match serde_json::from_str(&message) {
                            Ok(response) => response,
                            Err(_) => message
                        };
                        Ok(response)
                    }
                    Ok(None) => Err("No response received".into()),
                    Err(_) => Err("Response timeout".into()),
                }
            }
        }
    };
    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn request_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as syn::ItemFn);
    // Iterate over the inputs of the function

    // extract the first argument of the function to extract the response type
    let request_type_str = match input_fn.sig.inputs.first() {
        Some(syn::FnArg::Typed(pat_type)) => {
            match *pat_type.ty {
                syn::Type::Path(ref type_path) => {
                    if let Some(last_segment) = type_path.path.segments.last() {
                        last_segment.ident.to_string()
                    } else {
                        panic!("No response type found")
                    }
                },
                _ => panic!("No response type found")
            }
        },
        _ => panic!("No response type found")
    };

    

   
    let struct_name_str = format!("{}Handler", request_type_str);
    let struct_name = syn::Ident::new(&struct_name_str, proc_macro2::Span::call_site());
    let request_type = syn::Ident::new(&request_type_str, proc_macro2::Span::call_site());


    // Here, you would implement the logic to register the function
    // For simplicity, we're just going to return the function as-is
  // Return the original function without modifications
    let output = quote! {
        struct #struct_name {
            subscription: Option<MqttSubscription>,
        }


        impl RpcRequestHandler<#request_type> for #struct_name {
            fn start(&mut self) {
                let request_topic = format!(
                    "rpc/+/{}",
                    std::any::type_name::<#request_type>()
                    );
                let shared_topic = format!("$share/meshpulse/{}", request_topic);
                self.subscription = Some(MqttSubscription {
                    topic: request_topic.clone(),
                    id: uuid::Uuid::new_v4(),
                });

                let mut mqtt_client = MQTTCLIENT.write().unwrap();
                mqtt_client.client.subscribe(&shared_topic, QOS).unwrap();
                let topic = mqtt_client
                    .topics
                    .entry(request_topic.clone())
                    .or_insert(std::sync::Arc::new(std::sync::Mutex::new(
                                std::collections::HashMap::new(),
                                )));
                let mut topic = topic.lock().unwrap();
                topic.insert(
                    self.subscription.as_ref().unwrap().id,
                    Box::new(move |msg: paho_mqtt::Message| {
                        let payload = msg.payload_str().to_string();
                        let request: #request_type = serde_json::from_str(&payload).unwrap();
                        let response = Self::handle_request(request).unwrap();
                        let response_topic = format!("{}/response", msg.topic());
                        let response_msg = paho_mqtt::MessageBuilder::new()
                            .topic(response_topic)
                            .payload(response)
                            .qos(2)
                            .finalize();
                        let cli = &MQTTCLIENT.read().unwrap().client;
                        cli.publish(response_msg).unwrap();
                    }),
                );
            }
            fn handle_request(_request: TestRpcRequest) -> Result<String, Box<dyn std::error::Error>> {
                Ok("World".to_string())
            }

            fn stop(&self) {
                let mqtt_client = MQTTCLIENT.write().unwrap();
                mqtt_client
                    .client
                    .unsubscribe(&self.subscription.as_ref().unwrap().topic)
                    .unwrap();
            }
        }
    };

    output.into()
}
