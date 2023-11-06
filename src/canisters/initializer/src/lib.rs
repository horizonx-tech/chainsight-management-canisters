use std::{cell::RefCell, result};

use candid::{encode_args, encode_one, CandidType, Nat, Principal};
use ic_cdk::{
    api::{
        call::{self, CallResult},
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
    pub proxy: Principal,
    pub db: Principal,
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

#[derive(CandidType, serde::Deserialize, Clone, Copy)]
struct CycleManagement {
    initial_value: u128,
    refueling_value: u128,
    refueling_threashold: u128,
}

#[derive(CandidType, serde::Deserialize, Clone, Copy)]
struct CycleManagements {
    refueling_interval: u64,
    vault_intial_supply: u128,
    indexer: CycleManagement,
    db: CycleManagement,
    proxy: CycleManagement,
}

#[update]
async fn initialize(deployer: Principal, cycles: CycleManagements) -> InitializeOutput {
    let deposits_total = cycles.vault_intial_supply
        + cycles.indexer.initial_value
        + cycles.db.initial_value
        + cycles.proxy.initial_value;
    if deposits_total > ic_cdk::api::call::msg_cycles_accept128(deposits_total) {
        panic!("Acceptable cycles are less than the specified in parameters.")
    }

    let principal = ic_cdk::caller();

    let vault = create_new_canister(cycles.vault_intial_supply)
        .await
        .unwrap();
    install_vault(
        &vault,
        &principal,
        &deployer,
        deposits_total,
        cycles.refueling_interval,
    )
    .await
    .unwrap();
    after_install(&vault).await.unwrap();
    register(principal, vault).await;
    ic_cdk::println!(
        "Vault of {:?} installed at {:?}",
        principal.to_string(),
        vault.to_string()
    );

    let db = create_new_canister(cycles.db.initial_value).await.unwrap();
    install_db(db).await.unwrap();
    after_install(&db).await.unwrap();
    ic_cdk::println!(
        "DB of {:?} installed at {:?}",
        principal.to_string(),
        db.to_string()
    );
    init_db(db).await.unwrap();

    let proxy = create_new_canister(cycles.proxy.initial_value)
        .await
        .unwrap();
    install_proxy(proxy, principal, db).await.unwrap();
    after_install(&proxy).await.unwrap();
    ic_cdk::println!(
        "Proxy of {:?} installed at {:?}",
        principal.to_string(),
        proxy.to_string()
    );

    set_refuel_targets(cycles, vault, principal, db, proxy).await;

    InitializeOutput { proxy, db }
}

#[query]
fn get_registry() -> Principal {
    registry()
}

async fn install_vault(
    created: &Principal,
    canister: &Principal,
    deployer: &Principal,
    initial_supply: u128,
    refueling_interval: u64,
) -> CallResult<()> {
    let canister_id = created.clone();
    _install(
        canister_id,
        VAULT_WASM.to_vec(),
        encode_args((canister, deployer, initial_supply, refueling_interval)).unwrap(),
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
    update_settings(UpdateSettingsArgument {
        canister_id: canister_id,
        settings: CanisterSettings {
            controllers: Some(vec![
                ic_cdk::api::id(),
                // for Development
                ic_cdk::api::caller(),
            ]),
            compute_allocation: Some(Nat::from(0)),
            freezing_threshold: Some(Nat::from(2592000)),
            memory_allocation: Some(Nat::from(0)),
        },
    })
    .await
}

async fn create_new_canister(deposit: u128) -> CallResult<Principal> {
    let canister_id = create_canister(CreateCanisterArgument { settings: None })
        .await?
        .0
        .canister_id;
    deposit_cycles(CanisterIdRecord { canister_id }, deposit).await?;
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

async fn set_refuel_targets(
    cycles: CycleManagements,
    vault: Principal,
    indexer: Principal,
    db: Principal,
    proxy: Principal,
) {
    let res: CallResult<()> = ic_cdk::api::call::call(
        vault,
        "put_refuel_target",
        (
            indexer,
            cycles.indexer.refueling_value,
            cycles.indexer.refueling_threashold,
        ),
    )
    .await;
    res.unwrap();
    let res: CallResult<()> = ic_cdk::api::call::call(
        vault,
        "put_refuel_target",
        (
            db,
            cycles.db.refueling_value,
            cycles.db.refueling_threashold,
        ),
    )
    .await;
    res.unwrap();
    let res: CallResult<()> = ic_cdk::api::call::call(
        vault,
        "put_refuel_target",
        (
            proxy,
            cycles.proxy.refueling_value,
            cycles.proxy.refueling_threashold,
        ),
    )
    .await;
    res.unwrap();
}
