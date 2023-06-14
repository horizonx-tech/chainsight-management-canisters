use candid::Error;

use crate::message::{Message, MessageResult};

pub trait Caller {
    fn call(m: Message) -> Result<MessageResult, Error>;
}
