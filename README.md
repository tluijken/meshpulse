# Meshpulse

[![Cargo build and test](https://github.com/tluijken/meshpulse/actions/workflows/CARGO_TEST.yaml/badge.svg)](https://github.com/tluijken/meshpulse/actions/workflows/CARGO_TEST.yaml)
[![Crates.io](https://img.shields.io/crates/v/peshpulse.svg)](https://crates.io/crates/meshpulse)
[![Documentation](https://img.shields.io/badge/documentation-1)](https://docs.rs/meshpulse/latest/meshpulse/index.html)
[![GitHub issues](https://img.shields.io/github/issues/tluijken/meshpulse)]()
[![GitHub stars](https://img.shields.io/github/stars/tluijken/meshpulse)]()
[![GitHub license](https://img.shields.io/github/license/tluijken/meshpulse)]()

Meshpulse is a Rust crate designed to facilitate seamless communication between
microservices through events or RPC (Remote Procedure Call) calls. It abstracts
away the complexities of communication protocols such as MQTT, gRPC, AMQP, or
HTTP, allowing developers to focus on building robust, scalable microservices
without getting bogged down by the intricacies of inter-service communication.
Features

* Event-driven Communication: Easily exchange events between microservices,
  enabling real-time updates and event-driven architectures.
* RPC Calls: Make remote procedure calls between services with minimal overhead,
  ensuring efficient communication across the network.
* Protocol Agnostic: Supports multiple underlying protocols including MQTT,
  gRPC, AMQP, and HTTP, providing flexibility to choose the best protocol for
  your use case.
* Simplicity: Designed with simplicity in mind, Meshpulse abstracts away the
  complexities of communication protocols, making it easy to integrate into
  existing projects and workflows.
* Scalability: Built to scale, Meshpulse ensures smooth communication between
  microservices, even in highly distributed and complex environments.

# Installation

To use Meshpulse in your Rust project, simply add it as a dependency in your
Cargo.toml file:

```toml
[dependencies]
# To use with MQTT
meshpulse = { version = "0.2.0", features = ["mqtt"]}
```

## Usage

Here's a basic example demonstrating how to use Meshpulse for event-driven communication:

### Publishing
```rust
use meshpulse::prelude::*;

#[derive(Serialize, Deserialize, Event)]
struct TestEvent {
   message: String,
}

fn main() {
    let event = TestEvent {
        message: "hello".to_string(),
    };
    let result = event.publish();
    assert_eq!(result.is_ok(), true);
}
````

### Subscribing
```rust
use meshpulse::prelude::*;

#[derive(Serialize, Deserialize, Event)]
struct TestEvent {
  message: String,
}

fn main() {
    let sub_result = TestEvent::subscribe(|event| {
        println!("Received event: {:?}", event.message);
    });
    assert_eq!(sub_result.is_ok(), true);
    // When you're no longer interested in events...unsubscribe
    let unsub_result = sub_result.unwrap().unsubscribe();
    assert_eq!(unsub_result.is_ok(), true);
}
```

### RPC
To and execute an RPC request
```rust
use meshpulse::prelude::*;

#[derive(Serialize, Deserialize, RpcRequest)]
struct MultiplierRequest{
    number: i32,
    multiplier: i32
}

async fn main() {
    let request = MultiplierRequest {
        number: 5, 
        multiplier: 2
    };
    let response = request.request::<i32>().await.unwrap();
    assert_eq!(response, 10);
}
```

To subscribe to RPC requests and assign a handler function for requests:
```rust
use meshpulse::prelude::*;

fn handle_multiplier_request(request: MultiplierRequest) -> Result<i32, Box<dyn std::error::Error>> {
    Ok(request.number * request.multiplier)
}
let handler = RpcRequestHandler::start(handle_multiplier_request);
// keep the handler alive as long as required.
// To stop the handler
handler.stop();
```

### Configuration
To communicate with over MQTT, Meshpulse will need the following environment
variables

```env
MQTT_HOST='tcp://your-host:1883'
MQTT_USERNAME='your-username'
MQTT_PASSWORD='some secret passw0rd'
```

For more detailed usage instructions and examples, please refer to the
documentation. Contributing

Contributions are welcome! If you encounter any issues or have suggestions for
improvement, please feel free to open an issue or submit a pull request on the
GitHub repository. License

This project is licensed under the MIT License - see the LICENSE file for
details.
