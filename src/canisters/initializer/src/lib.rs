use std::cell::RefCell;

use candid::{encode_args, encode_one, CandidType, Nat, Principal};
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
    query, update,
};

#[derive(CandidType, serde::Deserialize, Clone, Copy)]
pub struct InitializeOutput {
    proxy: Principal,
    db: Principal,
}

#[cfg(debug_cfg)]
const VAULT_WASM: &[u8] =
    include_bytes!("../../../../target/wasm32-unknown-unknown/debug/vault.wasm");
#[cfg(not(debug_cfg))]
const VAULT_WASM: &[u8] =
    include_bytes!("../../../../target/wasm32-unknown-unknown/release/vault.wasm");
#[cfg(debug_cfg)]
const PROXY_WASM: &[u8] =
    include_bytes!("../../../../target/wasm32-unknown-unknown/debug/proxy.wasm");
#[cfg(not(debug_cfg))]
const PROXY_WASM: &[u8] =
    include_bytes!("../../../../target/wasm32-unknown-unknown/release/proxy.wasm");

const DB_WASM: &[u8] = include_bytes!("../../../../artifacts/Registry.wasm");

thread_local! {
    static REGISTRY: RefCell<Principal> = RefCell::new(Principal::anonymous());
}

fn registry() -> Principal {
    REGISTRY.with(|r| r.borrow().clone())
}

#[update]
async fn initialize() -> InitializeOutput {
    let principal = ic_cdk::caller();
    let vault = create_new_canister().await.unwrap();
    install_vault(&vault, &principal).await.unwrap();
    after_install(&vault).await.unwrap();
    register(principal, vault).await;
    let db = create_new_canister().await.unwrap();
    install_db(db).await.unwrap();
    after_install(&db).await.unwrap();
    let proxy = create_new_canister().await.unwrap();
    install_proxy(proxy, principal, db).await.unwrap();
    after_install(&proxy).await.unwrap();
    InitializeOutput { proxy, db }
}

#[query]
fn get_registry() -> Principal {
    registry()
}

async fn install_vault(created: &Principal, canister: &Principal) -> CallResult<()> {
    let canister_id = created.clone();
    _install(
        canister_id,
        VAULT_WASM.to_vec(),
        encode_one(canister.as_slice().to_vec()).unwrap(),
    )
    .await
}

async fn install_db(created: Principal) -> CallResult<()> {
    let canister_id = created.clone();
    _install(canister_id, DB_WASM.to_vec(), encode_args(()).unwrap()).await
}

async fn _install(canister_id: Principal, wasm_module: Vec<u8>, arg: Vec<u8>) -> CallResult<()> {
    install_code(InstallCodeArgument {
        mode: CanisterInstallMode::Reinstall,
        canister_id,
        wasm_module,
        arg,
    })
    .await
}

async fn install_proxy(created: Principal, target: Principal, db: Principal) -> CallResult<()> {
    let canister_id = created.clone();
    let registry = get_registry();
    _install(
        canister_id,
        PROXY_WASM.to_vec(),
        encode_args((registry, target, db)).unwrap(),
    )
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
fn set_registry(id: Principal) {
    REGISTRY.with(|registry| {
        *registry.borrow_mut() = id;
    });
}

async fn register(principal: Principal, vault: Principal) {
    let reg = registry();
    let _: CallResult<()> =
        ic_cdk::api::call::call(reg, "registerCanister", (principal, vault)).await;
}
