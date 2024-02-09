pub mod clients;

pub trait Publish {
    fn publish(&self) -> Result<(), Box<dyn std::error::Error>>;
}

pub trait Subscribe {
    type Event;
    fn subscribe(
        callback: impl FnMut(Self::Event) -> () + Send + 'static,
    ) -> Result<impl Subscription, Box<dyn std::error::Error>>;
}

pub trait Subscription: Sized {
    fn unsubscribe(self) -> Result<(), Box<dyn std::error::Error>>;
}

// prelude
pub mod prelude {
    pub use super::Publish;
    pub use super::Subscribe;
    pub use super::Subscription;
}

