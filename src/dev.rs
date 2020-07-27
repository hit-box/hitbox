pub use actix_cache_backend::{Backend, BackendError, Get, Set, Lock, LockStatus, Delete, DeleteStatus};

pub mod backend {
    use super::*;
    use actix::prelude::*;

    #[derive(Debug, Clone, PartialEq)]
    pub enum MockMessage {
        Get(Get),
        Set(Set),
        Delete(Delete),
        Lock(Lock),
    }

    pub struct MockBackend {
        pub messages: Vec<MockMessage>,
    }

    impl MockBackend {
        pub fn new() -> Self {
            MockBackend { messages: Vec::with_capacity(10) }
        }
    }

    impl Actor for MockBackend {
        type Context = Context<Self>;
    }

    impl Backend for MockBackend {
        type Actor = Self;
        type Context = Context<Self>;
    }

    impl Handler<Get> for MockBackend {
        type Result = <Get as Message>::Result;

        fn handle(&mut self, msg: Get, _: &mut Self::Context) -> Self::Result {
            self.messages.push(MockMessage::Get(msg));
            Ok(None)
        }
    }

    impl Handler<Set> for MockBackend {
        type Result = <Set as Message>::Result;

        fn handle(&mut self, msg: Set, _: &mut Self::Context) -> Self::Result {
            self.messages.push(MockMessage::Set(msg));
            Ok("".to_owned())
        }
    }

    impl Handler<Lock> for MockBackend {
        type Result = <Lock as Message>::Result;

        fn handle(&mut self, msg: Lock, _: &mut Self::Context) -> Self::Result {
            self.messages.push(MockMessage::Lock(msg));
            Ok(LockStatus::Locked)
        }
    }
    
    impl Handler<Delete> for MockBackend {
        type Result = <Delete as Message>::Result;

        fn handle(&mut self, msg: Delete, _: &mut Self::Context) -> Self::Result {
            self.messages.push(MockMessage::Delete(msg));
            Ok(DeleteStatus::Missing)
        }
    }

    #[derive(Message)]
    #[rtype(result = "GetMessagesResult")]
    pub struct GetMessages;

    #[derive(MessageResponse)]
    pub struct GetMessagesResult(pub Vec<MockMessage>);

    impl Handler<GetMessages> for MockBackend {
        type Result = GetMessagesResult;

        fn handle(&mut self, _msg: GetMessages, _: &mut Self::Context) -> Self::Result {
            GetMessagesResult(self.messages.clone())
        }
    }
}
