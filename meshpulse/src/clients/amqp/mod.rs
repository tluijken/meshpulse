pub mod client;
pub mod subscription;
use lazy_static::lazy_static;
use std::sync::RwLock;

lazy_static! {
    pub static ref AMQPCLIENT: RwLock<client::AMQPClient> ={
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            RwLock::new(client::AMQPClient::new().await)
        })
    };
}
