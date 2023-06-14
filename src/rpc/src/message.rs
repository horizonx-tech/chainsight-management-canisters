use candid::Principal;
use serde::{de::DeserializeOwned, Serialize};
#[derive(Debug)]
pub enum Error {
    InvalidPrincipal(Principal),
    InvalidRequest(String),
    InvalidContent(String),
    InvalidDestination(String),
}
type MessageContent = Vec<u8>;
type MethodName = String;

pub struct Message {
    content: MessageContent,
    recipient: Principal,
    method_name: MethodName,
}

pub struct MessageResult {
    reply: Vec<u8>,
}

impl MessageResult {
    pub fn reply<T>(&self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        match serde_json::from_slice(&self.reply) {
            Ok(content) => Ok(content),
            Err(e) => Err(Error::InvalidContent(e.to_string())),
        }
    }
}

impl Message {
    pub fn new<T>(content: T, recipient: Principal, method_name: &str) -> Result<Self, Error>
    where
        T: Serialize,
    {
        match serde_json::to_vec(&content) {
            Ok(content) => Ok(Message {
                content,
                recipient,
                method_name: method_name.to_string(),
            }),
            Err(e) => Err(Error::InvalidContent(e.to_string())),
        }
    }

    pub fn content<T>(&self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        match serde_json::from_slice(&self.content) {
            Ok(content) => Ok(content),
            Err(e) => Err(Error::InvalidContent(e.to_string())),
        }
    }
    pub fn recipient(&self) -> Principal {
        self.recipient
    }
}

#[cfg(test)]
pub mod tests {
    use serde::Deserialize;

    use super::*;
    #[test]
    fn test_new() {
        #[derive(Serialize, Deserialize)]
        struct TestStruct {
            uint: u32,
            string: String,
            vector: Vec<u8>,
        }
        let test_struct = TestStruct {
            uint: 42,
            string: "Hello, World!".to_string(),
            vector: vec![0, 1, 2, 3, 4, 5],
        };
        let recipient = Principal::anonymous();
        let message = Message::new(test_struct, recipient, "").unwrap();
        assert_eq!(message.recipient(), recipient);
        let content: TestStruct = message.content().unwrap();
        assert_eq!(content.uint, 42);
        assert_eq!(content.string, "Hello, World!".to_string());
        assert_eq!(content.vector, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_reply() {
        #[derive(Deserialize, Serialize)]
        struct TestStruct {
            uint: u32,
            string: String,
            vector: Vec<u8>,
        }
        let test_struct = TestStruct {
            uint: 42,
            string: "Hello, World!".to_string(),
            vector: vec![0, 1, 2, 3, 4, 5],
        };
        let result = MessageResult {
            reply: serde_json::to_vec(&test_struct).unwrap(),
        };
        let content: TestStruct = result.reply().unwrap();
        assert_eq!(content.uint, 42);
        assert_eq!(content.string, "Hello, World!".to_string());
        assert_eq!(content.vector, vec![0, 1, 2, 3, 4, 5]);
    }
}
