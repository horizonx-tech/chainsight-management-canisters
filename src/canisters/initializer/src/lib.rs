use candid::{candid_method, encode_args, Principal};
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
    query, update, pre_upgrade, storage, post_upgrade,
};
use std::cell::RefCell;

mod types;
use types::{InitializeOutput, CycleManagements, RefuelTarget, RegisteredCanisterInRegistry, ComponentInfoFromProxy};

use crate::types::UpgradeStableState;

const VAULT_WASM: &[u8] = include_bytes!("../../../../artifacts/vault.wasm.gz");
const PROXY_WASM: &[u8] = include_bytes!("../../../../artifacts/proxy.wasm.gz");
const DB_WASM: &[u8] = include_bytes!("../../../../artifacts/registry.wasm.gz");

thread_local! {
    static REGISTRY: RefCell<Principal> = RefCell::new(Principal::anonymous());
}

#[query]
#[candid_method(query)]
fn get_registry() -> Principal {
    REGISTRY.with(|r| r.borrow().clone())
}

#[update]
#[candid_method(update)]
fn set_registry(id: Principal) {
    REGISTRY.with(|registry| *registry.borrow_mut() = id);
}

#[update]
#[candid_method(update)]
async fn initialize(deployer: Principal, cycles: CycleManagements) -> InitializeOutput {
    let deposits_total = cycles.initial_supply();
    if deposits_total > ic_cdk::api::call::msg_cycles_accept128(deposits_total) {
        panic!("Acceptable cycles are less than the specified in parameters.")
    }

    let principal = ic_cdk::caller();

    let vault = create_new_canister(cycles.vault_intial_supply)
        .await
        .unwrap();
    let controllers = &vec![deployer, vault, ic_cdk::api::id()];

    after_install(&principal, controllers).await.unwrap();

    let db = create_new_canister(cycles.db.initial_supply).await.unwrap();
    install_db(db).await.unwrap();
    after_install(&db, controllers).await.unwrap();
    ic_cdk::println!(
        "DB of {:?} installed at {:?}",
        principal.to_string(),
        db.to_string()
    );
    init_db(db).await.unwrap();

    let proxy = create_new_canister(cycles.proxy.initial_supply)
        .await
        .unwrap();
    install_proxy(proxy, principal, db, vault).await.unwrap();
    after_install(&proxy, controllers).await.unwrap();
    ic_cdk::println!(
        "Proxy of {:?} installed at {:?}",
        principal.to_string(),
        proxy.to_string()
    );

    install_vault(&vault, &principal, &db, &proxy, &deployer, &cycles)
        .await
        .unwrap();
    after_install(&vault, controllers).await.unwrap();
    register(principal, vault).await;
    ic_cdk::println!(
        "Vault of {:?} installed at {:?}",
        principal.to_string(),
        vault.to_string()
    );

    InitializeOutput { vault, proxy, db }
}

async fn install_vault(
    created: &Principal,
    indexer: &Principal,
    db: &Principal,
    proxy: &Principal,
    deployer: &Principal,
    cycles: &CycleManagements,
) -> CallResult<()> {
    let canister_id = created.clone();
    _install(
        canister_id,
        VAULT_WASM.to_vec(),
        encode_args((
            indexer,
            deployer,
            cycles.initial_supply(),
            cycles.refueling_interval,
            vec![
                RefuelTarget {
                    id: indexer.clone(),
                    amount: cycles.indexer.refueling_amount,
                    threshold: cycles.indexer.refueling_threshold,
                },
                RefuelTarget {
                    id: db.clone(),
                    amount: cycles.db.refueling_amount,
                    threshold: cycles.db.refueling_threshold,
                },
                RefuelTarget {
                    id: proxy.clone(),
                    amount: cycles.proxy.refueling_amount,
                    threshold: cycles.proxy.refueling_threshold,
                },
            ],
            vec![
                (indexer.clone(), cycles.indexer.initial_supply),
                (db.clone(), cycles.db.initial_supply),
                (proxy.clone(), cycles.proxy.initial_supply),
            ],
        ))
        .unwrap(),
    )
    .await
}

async fn install_db(created: Principal) -> CallResult<()> {
    let canister_id = created.clone();
    _install(canister_id, DB_WASM.to_vec(), encode_args(()).unwrap()).await
}

async fn init_db(db: Principal) -> CallResult<()> {
    let out: CallResult<()> = ic_cdk::api::call::call(db, "init", ()).await;
    out
}

async fn install_proxy(created: Principal, target: Principal, db: Principal, vault: Principal) -> CallResult<()> {
    let canister_id = created.clone();
    let registry = get_registry();
    _install(
        canister_id,
        PROXY_WASM.to_vec(),
        encode_args((registry, target, db, vault)).unwrap(),
    )
    .await
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

async fn after_install(canister_id: &Principal, controllers: &Vec<Principal>) -> CallResult<()> {
    update_settings(UpdateSettingsArgument {
        canister_id: canister_id.clone(),
        settings: CanisterSettings {
            controllers: Some(controllers.clone()),
            compute_allocation: None,
            freezing_threshold: None,
            memory_allocation: None,
        },
    })
    .await
}

async fn create_new_canister(deposit: u128) -> CallResult<Principal> {
    let canister_id = create_canister(
        CreateCanisterArgument { settings: None },
        100_000_000_000u128 // NOTE: from https://github.com/dfinity/cdk-rs/blob/a8454cb37420c200c7b224befd6f68326a01442e/src/ic-cdk/src/api/management_canister/main/mod.rs#L17-L32
    )
    .await?
    .0
    .canister_id;
    deposit_cycles(CanisterIdRecord { canister_id }, deposit).await?;
    Ok(canister_id)
}

async fn register(principal: Principal, vault: Principal) {
    let reg = get_registry();
    let _: CallResult<()> =
        ic_cdk::api::call::call(reg, "registerCanister", (principal, vault)).await;
}

#[update]
#[candid_method(update)]
async fn upgrade_proxies() {
    let caller_proxy = ic_cdk::caller();
    let registry = get_registry();
    let ComponentInfoFromProxy { target: component_canister, vault, db } = get_component_info_of_proxy(caller_proxy.clone())
        .await
        .expect("Failed to call 'target' to Proxy")
        .0;

    // check if caller is a registered proxy
    let res = get_registered_canister_in_db(registry, component_canister).await.expect("Failed to call 'getRegisteredCanister' to Registry");
    assert!(res.0.is_some(), "Caller is not a registered proxy");

    // install_code with upgrade mode
    let _ = install_for_upgrade(db, DB_WASM.to_vec(), vec![]).await.expect("Failed to upgrade DB for proxy");
    let _ = install_for_upgrade(vault, VAULT_WASM.to_vec(), vec![]).await.expect("Failed to upgrade Vault for proxy");
    let _ = install_for_upgrade(caller_proxy, PROXY_WASM.to_vec(), vec![]).await.expect("Failed to upgrade Proxy for proxy");
}

async fn install_for_upgrade(canister_id: Principal, wasm_module: Vec<u8>, arg: Vec<u8>) -> CallResult<()> {
    install_code(InstallCodeArgument {
        mode: CanisterInstallMode::Upgrade,
        canister_id,
        wasm_module,
        arg,
    })
    .await
}

async fn get_component_info_of_proxy(proxy: Principal) -> CallResult<(ComponentInfoFromProxy,)> {
    let out: CallResult<(ComponentInfoFromProxy,)> = ic_cdk::api::call::call(proxy, "get_component_info", ()).await;
    out
}

async fn get_registered_canister_in_db(db: Principal, target: Principal) -> CallResult<(Option<RegisteredCanisterInRegistry>,)> {
    let out: CallResult<(Option<RegisteredCanisterInRegistry>,)> = ic_cdk::api::call::call(db, "getRegisteredCanister", (target,)).await;
    out
}

#[pre_upgrade]
fn pre_upgrade() {
    ic_cdk::println!("start: pre_upgrade");

    let state = UpgradeStableState {
        registry: get_registry(),
    };
    storage::stable_save((state,)).expect("Failed to save stable state");

    ic_cdk::println!("finish: pre_upgrade");
}

#[post_upgrade]
fn post_upgrade() {
    ic_cdk::println!("start: post_upgrade");

    let (state,): (UpgradeStableState,) = storage::stable_restore().expect("Failed to restore stable state");
    set_registry(state.registry);

    ic_cdk::println!("finish: post_upgrade");
}

#[cfg(test)]
mod tests {
    use super::*;
    candid::export_service!();

    #[test]
    fn generate_candid() {
        std::fs::write("initializer.did", __export_service()).unwrap();
    }
}
