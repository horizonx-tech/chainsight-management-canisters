use std::{cell::RefCell, str::FromStr};

use candid::{encode_one, Nat, Principal};
use ic_cdk::{
    api::{
        call::CallResult,
        management_canister::{
            main::{
                create_canister, deposit_cycles, install_code, update_settings,
                CanisterInstallMode, CreateCanisterArgument, InstallCodeArgument,
                UpdateSettingsArgument,
            },
            provisional::{CanisterIdRecord, CanisterSettings},
        },
    },
    update,
};

#[cfg(debug_cfg)]
const VAULT_WASM: &[u8] =
    include_bytes!("../../../../target/wasm32-unknown-unknown/debug/vault.wasm");
#[cfg(not(debug_cfg))]
const VAULT_WASM: &[u8] =
    include_bytes!("../../../../target/wasm32-unknown-unknown/release/vault.wasm");

thread_local! {
    static REGISTRY: RefCell<String> = RefCell::new(String::new());
}

fn registry() -> Principal {
    REGISTRY.with(|registry| Principal::from_str(&registry.borrow()).unwrap())
}

#[update]
async fn deploy_vault_of(principal: Principal) -> Principal {
    let p = create_new_canister().await.unwrap();
    install(&p, &principal).await.unwrap();
    after_install(&p).await.unwrap();
    register(principal, p).await;
    p
}

#[update]
async fn get_proxy() -> Principal {
    _get_proxy().await
}

async fn _get_proxy() -> Principal {
    let px: CallResult<(Principal,)> = ic_cdk::api::call::call(registry(), "getProxy", ()).await;
    px.unwrap().0
}

async fn install(created: &Principal, canister: &Principal) -> CallResult<()> {
    let canister_id = created.clone();

    install_code(InstallCodeArgument {
        mode: CanisterInstallMode::Reinstall,
        canister_id,
        wasm_module: VAULT_WASM.to_vec(),
        arg: encode_one(canister.as_slice().to_vec()).unwrap(),
    })
    .await
}

async fn after_install(canister_id: &Principal) -> CallResult<()> {
    let canister_id = canister_id.clone();
    deposit_cycles(CanisterIdRecord { canister_id }, 300_000_000_000).await?;

    update_settings(UpdateSettingsArgument {
        canister_id: canister_id,
        settings: CanisterSettings {
            controllers: Some(vec![ic_cdk::api::id()]),
            compute_allocation: Some(Nat::from(0)),
            freezing_threshold: Some(Nat::from(2592000)),
            memory_allocation: Some(Nat::from(0)),
        },
    })
    .await
}

async fn create_new_canister() -> CallResult<Principal> {
    let canister_id = create_canister(CreateCanisterArgument { settings: None })
        .await?
        .0
        .canister_id;
    Ok(canister_id)
}

#[update]
fn set_registry(id: String) {
    Principal::from_str(&id).unwrap();
    REGISTRY.with(|registry| {
        *registry.borrow_mut() = id;
    });
}

async fn register(principal: Principal, vault: Principal) {
    let _: CallResult<()> =
        ic_cdk::api::call::call(registry(), "registerCanister", (principal, vault)).await;
}
