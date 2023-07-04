use std::{error::Error, path::Path};

use candid::{CandidType, Nat};
use client::Client;
use ic_agent::{export::Principal, identity::Secp256k1Identity, Agent};
use serde::Deserialize;

mod client;
#[derive(CandidType)]
struct Argument {
    amount: Option<Nat>,
}
#[derive(CandidType, Deserialize)]
struct CreateCanisterResult {
    canister_id: Principal,
}

//pub const INITIALIZER_CANISTER_ID: &str = "c2lt4-zmaaa-aaaaa-qaaiq-cai";
//pub const PROXY_CANISTER_ID: &str = "c2lt4-zmaaa-aaaaa-qaaiq-cai";
//pub const REGISTRY_CANISTER_ID: &str = "cuj6u-c4aaa-aaaaa-qaajq-cai";
const INITIALIZER_WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/initializer.wasm");
const PROXY_WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/proxy.wasm");
const REGISTRY_WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/registry.wasm");

pub async fn deploy(agent: Agent) -> Result<(), Box<dyn Error>> {
    let client = Client::new(agent);

    for canister in [(INITIALIZER_WASM), (PROXY_WASM), (REGISTRY_WASM)] {
        let deployed = client.create_canister().await?;
        client.install_code(deployed, canister).await?;
        println!("Deployed {}", deployed)
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
                ReqwestHttpReplicaV2Transport::create("http://localhost:57862")
                    .expect("transport error"),
            )
            .with_identity(get_dfx_identity("default"))
            .build()
            .unwrap();
        deploy(agent).await.unwrap();
    }
}
