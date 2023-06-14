use std::borrow::Cow;

use candid::{CandidType, Decode, Encode, Principal};
use ic_stable_structures::{BoundedStorable, Storable};
use serde::Deserialize;

#[derive(CandidType, Deserialize)]
pub struct ChainsightCanister {
    registered_at: u64,
    last_seen_at: u64,
    created_by: ID,
}

impl ChainsightCanister {
    pub fn new(created_by: ID) -> Self {
        Self {
            registered_at: ic_cdk::api::time(),
            last_seen_at: ic_cdk::api::time(),
            created_by,
        }
    }
}

#[derive(CandidType, Deserialize, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct ID {
    principal: String,
}

impl ID {
    pub fn new(principal: Principal) -> Self {
        Self {
            principal: principal.to_text(),
        }
    }
    pub fn to_principal(&self) -> Principal {
        Principal::from_text(&self.principal).unwrap()
    }
}

impl Storable for ID {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}
impl BoundedStorable for ID {
    const MAX_SIZE: u32 = 10;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for ChainsightCanister {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}
impl BoundedStorable for ChainsightCanister {
    const MAX_SIZE: u32 = 10;
    const IS_FIXED_SIZE: bool = false;
}
