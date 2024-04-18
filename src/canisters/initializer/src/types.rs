use candid::{CandidType, Principal};

#[derive(CandidType, serde::Deserialize, Clone, Copy)]
pub struct InitializeOutput {
    pub vault: Principal,
    pub proxy: Principal,
    pub db: Principal,
}


#[derive(CandidType, serde::Deserialize, Clone, Copy)]
pub struct CycleManagement {
    pub initial_supply: u128,
    pub refueling_amount: u128,
    pub refueling_threshold: u128,
}

#[derive(CandidType, serde::Deserialize, Clone, Copy)]
pub struct CycleManagements {
    pub refueling_interval: u64,
    pub vault_intial_supply: u128,
    pub indexer: CycleManagement,
    pub db: CycleManagement,
    pub proxy: CycleManagement,
}
impl CycleManagements {
    pub fn initial_supply(&self) -> u128 {
        self.vault_intial_supply
            + self.indexer.initial_supply
            + self.db.initial_supply
            + self.proxy.initial_supply
    }
}

#[derive(CandidType, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct RefuelTarget {
    pub id: Principal,
    pub amount: u128,
    pub threshold: u128,
}

#[derive(Clone, Debug, candid::CandidType, candid::Deserialize, serde::Serialize)]
pub struct ComponentInfoFromProxy {
    pub target: Principal,
    pub vault: Principal,
    pub db: Principal,
}

#[derive(CandidType, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct RegisteredCanisterInRegistry {
    pub principal: Principal,
    pub vault: Principal,
}

#[derive(CandidType, serde::Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub cycles: u128,
}

#[derive(CandidType, serde::Deserialize)]
pub struct UpgradeStableState {
    pub registry: Principal,
}
