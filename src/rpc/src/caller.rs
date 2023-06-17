use async_trait::async_trait;
use ic_cdk::api::call::CallResult;

use crate::message::{Message, MessageResult};

#[async_trait]
pub trait Caller {
    async fn call(&self, m: Message) -> CallResult<MessageResult>;
}
