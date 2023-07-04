use std::{error::Error, slice::Iter};

use candid::{CandidType, Decode, Encode, Nat, Principal};
use ic_agent::Agent;
use serde::{Deserialize, Serialize};

pub struct Client {
    agent: Agent,
}
/// The mode with which a canister is installed.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Eq, Hash, CandidType, Copy)]
pub enum CanisterInstallMode {
    /// A fresh install of a new canister.
    #[serde(rename = "install")]
    Install,
    /// Reinstalling a canister that was already installed.
    #[serde(rename = "reinstall")]
    Reinstall,
    /// Upgrade an existing canister.
    #[serde(rename = "upgrade")]
    Upgrade,
}
impl Default for CanisterInstallMode {
    fn default() -> Self {
        CanisterInstallMode::Install
    }
}
#[derive(CandidType, Clone, Deserialize)]
pub struct CanisterSettings {
    controller: Option<Principal>,
    compute_allocation: Option<Nat>,
    memory_allocation: Option<Nat>,
    freezing_threshold: Option<Nat>,
}
#[derive(Clone, CandidType, Deserialize, Debug)]
pub struct InstallCodeArgs {
    pub mode: CanisterInstallMode,
    pub canister_id: Principal,
    #[serde(with = "serde_bytes")]
    pub wasm_module: Vec<u8>,
    pub arg: Vec<u8>,
    pub compute_allocation: Option<Nat>,
    pub memory_allocation: Option<Nat>,
    pub query_allocation: Option<Nat>,
    pub sender_canister_version: Option<u64>,
}

#[derive(CandidType, Deserialize)]
pub struct CreateCanisterArgs {
    cycles: u64,
    settings: CanisterSettings,
}
#[derive(CandidType, Deserialize)]
pub struct CreateResult {
    canister_id: Principal,
}
impl CanisterInstallMode {
    pub fn iter() -> Iter<'static, CanisterInstallMode> {
        static MODES: [CanisterInstallMode; 3] = [
            CanisterInstallMode::Install,
            CanisterInstallMode::Reinstall,
            CanisterInstallMode::Upgrade,
        ];
        MODES.iter()
    }
}

impl Client {
    pub fn new(agent: Agent) -> Self {
        Self { agent }
    }
    pub async fn create_canister(
        &self,
        effective_canister_id: Principal,
    ) -> Result<Principal, Box<dyn Error>> {
        #[derive(CandidType)]
        struct Argument {
            amount: Option<Nat>,
        }
        #[derive(CandidType, Deserialize)]
        struct CreateCanisterResult {
            canister_id: Principal,
        }

        self.agent.fetch_root_key().await?;
        let management_canister_id = Principal::management_canister();
        let response = self
            .agent
            .update(
                &management_canister_id,
                "provisional_create_canister_with_cycles",
            )
            .with_effective_canister_id(effective_canister_id)
            .with_arg(&Encode!(&Argument { amount: None })?)
            .call_and_wait()
            .await?;

        let result = Decode!(response.as_slice(), CreateCanisterResult)?;
        let canister_id: Principal = result.canister_id;
        Ok(canister_id)
    }
    pub async fn install_code(
        &self,
        canister_id: Principal,
        wasm: &[u8],
    ) -> Result<(), Box<dyn Error>> {
        self.agent
            .update(&Principal::management_canister(), "install_code")
            .with_effective_canister_id(canister_id)
            .with_arg(&Encode!(&InstallCodeArgs {
                mode: CanisterInstallMode::Install,
                canister_id,
                wasm_module: wasm.to_vec(),
                arg: b"init".to_vec(),
                compute_allocation: None,
                memory_allocation: None,
                query_allocation: None,
                sender_canister_version: None,
            })?)
            .call_and_wait()
            .await?;
        Ok(())
    }
}
