use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize)]
pub struct CanisterRegisterInput {
    pub principal: Principal,
    pub craeted_by: Principal,
}
impl CanisterRegisterInput {
    pub fn new(principal: Principal, craeted_by: Principal) -> Self {
        Self {
            principal,
            craeted_by,
        }
    }
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct CallMetric {
    pub canister_id: Principal,
    pub method_name: String,
    pub called_by: Principal,
    pub time: u64,
    pub cycles: u64,
    pub bytes: u64,
    pub cycles_balance: u64,
}

impl CallMetric {
    pub fn new(
        canister_id: Principal,
        method_name: String,
        called_by: Principal,
        cycles: u64,
        bytes: u64,
        cycles_balance: u64,
    ) -> Self {
        Self {
            canister_id,
            method_name,
            called_by,
            time: ic_cdk::api::time(),
            cycles,
            bytes,
            cycles_balance,
        }
    }
}
