use std::{error::Error, path::Path, str::FromStr};

use candid::{CandidType, Decode, Encode, Nat};
use ic_agent::{export::Principal, identity::Secp256k1Identity, Agent};
use serde::Deserialize;

#[derive(CandidType)]
struct Argument {
    amount: Option<Nat>,
}
#[derive(CandidType, Deserialize)]
struct CreateCanisterResult {
    canister_id: Principal,
}

pub const INITIALIZER_CANISTER_ID: &str = "c2lt4-zmaaa-aaaaa-qaaiq-cai";
pub const PROXY_CANISTER_ID: &str = "c2lt4-zmaaa-aaaaa-qaaiq-cai";
pub const REGISTRY_CANISTER_ID: &str = "cuj6u-c4aaa-aaaaa-qaajq-cai";

pub async fn deploy(agent: Agent) -> Result<(), Box<dyn Error>> {
    agent.fetch_root_key().await?;
    println!("!!!test");
    let management_canister_id = Principal::from_text("aaaaa-aa")?;
    println!("!!!management canister id:{}", management_canister_id);

    for canister_id in [
        INITIALIZER_CANISTER_ID,
        PROXY_CANISTER_ID,
        REGISTRY_CANISTER_ID,
    ] {
        let principal = Principal::from_str(canister_id)?;
        let response = agent
            .update(
                &management_canister_id,
                "provisional_create_canister_with_cycles",
            )
            .with_effective_canister_id(principal)
            .with_arg(&Encode!(&Argument { amount: None })?)
            .call_and_wait()
            .await?;
        let result = Decode!(response.as_slice(), CreateCanisterResult)?;
        let canister_id: Principal = result.canister_id;
    }

    Ok(())
}
pub fn get_dfx_identity(name: &str) -> Secp256k1Identity {
    let home_dir = dirs::home_dir().expect("Failed to get home directory");
    let pem_file_path = home_dir.join(Path::new(&format!(
        ".config/dfx/identity/{}/identity.pem",
        name
    )));
    Secp256k1Identity::from_pem_file(pem_file_path).expect("Failed to create identity")
}
#[cfg(test)]
mod tests {

    use super::*;
    use ic_agent::agent::http_transport::ReqwestHttpReplicaV2Transport;

    #[tokio::test]
    async fn test_deploy() {
        let agent = Agent::builder()
            .with_transport(
                ReqwestHttpReplicaV2Transport::create("http://localhost:64218")
                    .expect("transport error"),
            )
            .with_identity(get_dfx_identity("default"))
            .build()
            .unwrap();
        deploy(agent).await.unwrap();
    }
}
