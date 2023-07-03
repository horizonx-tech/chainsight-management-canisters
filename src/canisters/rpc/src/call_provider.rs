use async_trait::async_trait;
use candid::Principal;
use ic_cdk::api::call::{self, CallResult};

use crate::{
    caller::Caller,
    message::{Message, MessageCallResult, MessageResult},
};

pub struct CallProvider {
    proxy: Principal,
}

impl CallProvider {
    pub fn new(proxy: Principal) -> Self {
        Self { proxy }
    }
}

#[async_trait]
impl Caller for CallProvider {
    async fn call(&self, m: Message) -> CallResult<MessageResult> {
        let result: MessageCallResult = call::call(
            self.proxy,
            "proxy_call",
            (ic_cdk::caller(), m.recipient, m.method_name, m.content),
        )
        .await;
        match result {
            Ok(result) => match result.0 {
                Ok(result) => Ok(MessageResult::new(result.0)),
                Err(err) => {
                    ic_cdk::println!("Error: {:?}", err);
                    Err(err)
                }
            },
            Err(err) => Err(err),
        }
    }
}
