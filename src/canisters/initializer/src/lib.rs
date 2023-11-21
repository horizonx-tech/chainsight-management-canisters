use candid::{candid_method, encode_args, CandidType, Principal};
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
use std::cell::RefCell;

#[derive(CandidType, serde::Deserialize, Clone, Copy)]
pub struct InitializeOutput {
    pub vault: Principal,
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
    initial_supply: u128,
    refueling_amount: u128,
    refueling_threshold: u128,
}

#[derive(CandidType, serde::Deserialize, Clone, Copy)]
struct CycleManagements {
    refueling_interval: u64,
    vault_intial_supply: u128,
    indexer: CycleManagement,
    db: CycleManagement,
    proxy: CycleManagement,
}

impl CycleManagements {
    fn initial_supply(&self) -> u128 {
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
    install_proxy(proxy, principal, db).await.unwrap();
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

#[query]
fn get_registry() -> Principal {
    registry()
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

#[cfg(test)]
mod tests {
    use super::*;
    candid::export_service!();

    #[test]
    fn gen_candid() {
        std::fs::write("initializer.did", __export_service()).unwrap();
    }
}
