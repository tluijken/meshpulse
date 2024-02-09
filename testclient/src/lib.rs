pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use meshpulse::prelude::*;
    use meshpulse::clients::mqtt;

    #[derive(Serialize, Deserialize, Event)]
    struct TestEvent {
        message: String,
    }
    struct Test {

    }

    impl Test{
        fn test(){
            let event = TestEvent {
                message: "hello".to_string(),
            };
            let result = event.publish();
        }
    }
}
