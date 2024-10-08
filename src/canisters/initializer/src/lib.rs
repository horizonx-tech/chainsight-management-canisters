use candid::{candid_method, encode_args, Principal};
use cmc::{
    cmc,
    types::{CreateCanisterArg, CreateCanisterResult, SubnetSelection},
};
use ic_cdk::{
    api::{
        call::CallResult,
        management_canister::{
            main::{
                canister_status, create_canister, deposit_cycles, install_code, update_settings,
                CanisterInstallMode, CanisterStatusResponse, CreateCanisterArgument,
                InstallCodeArgument, UpdateSettingsArgument,
            },
            provisional::{CanisterIdRecord, CanisterSettings},
        },
    },
    caller, post_upgrade, pre_upgrade, query, storage, update,
};
use ic_cdk_timers::TimerId;
use std::cell::RefCell;

mod cmc;
mod types;
use types::{
    ComponentInfoFromProxy, CycleManagements, InitializeOutput, MetricsSnapshot, RefuelTarget,
    RegisteredCanisterInRegistry,
};

use crate::types::UpgradeStableState;

const VAULT_WASM: &[u8] = include_bytes!("../../../../artifacts/vault.wasm.gz");
const PROXY_WASM: &[u8] = include_bytes!("../../../../artifacts/proxy.wasm.gz");
const DB_WASM: &[u8] = include_bytes!("../../../../artifacts/registry.wasm.gz");

thread_local! {
    static REGISTRY: RefCell<Principal> = RefCell::new(Principal::anonymous());

    static METRIC_TIMER_ID: RefCell<Option<(TimerId, u64)>> = RefCell::new(None); // with interval_secs
    static METRICS: RefCell<Vec<MetricsSnapshot>> = RefCell::new(Vec::new());
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
async fn initialize(
    deployer: Principal,
    cycles: CycleManagements,
    subnet: Option<Principal>,
) -> InitializeOutput {
    let deposits_total = cycles.initial_supply();
    if deposits_total > ic_cdk::api::call::msg_cycles_accept128(deposits_total) {
        panic!("Acceptable cycles are less than the specified in parameters.")
    }

    let principal = ic_cdk::caller();

    let vault = create_new_canister(cycles.vault_intial_supply, subnet)
        .await
        .unwrap();
    let controllers = &vec![deployer, vault, ic_cdk::api::id()];
    update_controllers_for_canister(&principal, controllers)
        .await
        .unwrap();
    update_controllers_for_canister(&vault, controllers)
        .await
        .unwrap();

    let db = create_new_canister(cycles.db.initial_supply, subnet)
        .await
        .unwrap_or_else(|_| {
            panic!(
                "{}",
                format!(
                    "Failed to deploy db. deployed canisters: vault = {:?}",
                    vault.to_text()
                )
            )
        });
    let err_msg = format!(
        "Failed to initialize db. deployed canisters: vault = {:?}, db = {:?}",
        vault.to_text(),
        db.to_text()
    );
    update_controllers_for_canister(&db, controllers)
        .await
        .unwrap_or_else(|_| panic!("{}", &err_msg));
    install_db(db)
        .await
        .unwrap_or_else(|_| panic!("{}", &err_msg));
    ic_cdk::println!(
        "DB of {:?} installed at {:?}",
        principal.to_string(),
        db.to_string()
    );
    init_db(db).await.unwrap_or_else(|_| panic!("{}", &err_msg));

    let proxy = create_new_canister(cycles.proxy.initial_supply, subnet)
        .await
        .unwrap_or_else(|_| {
            panic!(
                "{}",
                &format!(
                    "Failed to deploy proxy. deployed canisters: vault = {:?}, db = {:?}",
                    vault.to_text(),
                    db.to_text()
                )
            )
        });
    let err_msg = format!(
        "Failed to initialize proxy. deployed canisters: vault = {:?}, db = {:?}, proxy = {:?}",
        vault.to_text(),
        db.to_text(),
        proxy.to_text()
    );
    update_controllers_for_canister(&proxy, controllers)
        .await
        .unwrap_or_else(|_| panic!("{}", &err_msg));
    install_proxy(proxy, principal, db, vault)
        .await
        .unwrap_or_else(|_| panic!("{}", &err_msg));
    ic_cdk::println!(
        "Proxy of {:?} installed at {:?}",
        principal.to_string(),
        proxy.to_string()
    );

    let err_msg = format!(
        "Failed to initialize vault. deployed canisters: vault = {:?}, db = {:?}, proxy = {:?}",
        vault.to_text(),
        db.to_text(),
        proxy.to_text()
    );
    install_vault(&vault, &principal, &db, &proxy, &deployer, &cycles)
        .await
        .unwrap_or_else(|_| panic!("{}", &err_msg));
    register_canister_of_registry(principal, vault)
        .await
        .unwrap_or_else(|_| panic!("{}", &err_msg));
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

async fn install_proxy(
    created: Principal,
    target: Principal,
    db: Principal,
    vault: Principal,
) -> CallResult<()> {
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

async fn update_controllers_for_canister(
    canister_id: &Principal,
    controllers: &Vec<Principal>,
) -> CallResult<()> {
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

async fn create_new_canister(deposit: u128, subnet: Option<Principal>) -> CallResult<Principal> {
    let cycles = 100_000_000_000u128; // NOTE: from https://github.com/dfinity/cdk-rs/blob/a8454cb37420c200c7b224befd6f68326a01442e/src/ic-cdk/src/api/management_canister/main/mod.rs#L17-L32
    
    if subnet.is_none() {
        let result = create_canister(CreateCanisterArgument { settings: None }, cycles)
            .await?
            .0;
        return Ok(result.canister_id);
    }

    let result = cmc()
        .create_canister(
            CreateCanisterArg {
                subnet_selection: Some(SubnetSelection::Subnet {
                    subnet: subnet.unwrap(),
                }),
                settings: None,
                subnet_type: None,
            },
            cycles,
        )
        .await?
        .0;
    match result {
        CreateCanisterResult::Ok(canister_id) => {
            deposit_cycles(CanisterIdRecord { canister_id }, deposit).await?;
            Ok(canister_id)
        }
        CreateCanisterResult::Err(err) => match err {
            cmc::types::CreateCanisterError::Refunded {
                create_error,
                refund_amount,
            } => {
                ic_cdk::trap(&format!(
                    "Failed to create canister: {:?}, refunded amount: {:?}",
                    create_error, refund_amount
                ));
            }
        },
    }
}

async fn register_canister_of_registry(principal: Principal, vault: Principal) -> CallResult<()> {
    let reg = get_registry();
    ic_cdk::api::call::call(reg, "registerCanister", (principal, vault)).await
}

#[update]
#[candid_method(update)]
async fn upgrade_proxies() {
    let caller_proxy = ic_cdk::caller();
    let registry = get_registry();
    let ComponentInfoFromProxy {
        target: component_canister,
        vault,
        db,
    } = get_component_info_of_proxy(caller_proxy.clone())
        .await
        .expect("Failed to call 'target' to Proxy")
        .0;

    // check if caller is a registered proxy
    let res = get_registered_canister_in_db(registry, component_canister)
        .await
        .expect("Failed to call 'getRegisteredCanister' to Registry");
    assert!(res.0.is_some(), "Caller is not a registered proxy");

    // install_code with upgrade mode
    let _ = install_for_upgrade(db, DB_WASM.to_vec(), vec![])
        .await
        .expect("Failed to upgrade DB for proxy");
    let _ = install_for_upgrade(vault, VAULT_WASM.to_vec(), vec![])
        .await
        .expect("Failed to upgrade Vault for proxy");
    let _ = install_for_upgrade(caller_proxy, PROXY_WASM.to_vec(), vec![])
        .await
        .expect("Failed to upgrade Proxy for proxy");
}

async fn install_for_upgrade(
    canister_id: Principal,
    wasm_module: Vec<u8>,
    arg: Vec<u8>,
) -> CallResult<()> {
    install_code(InstallCodeArgument {
        mode: CanisterInstallMode::Upgrade,
        canister_id,
        wasm_module,
        arg,
    })
    .await
}

async fn get_component_info_of_proxy(proxy: Principal) -> CallResult<(ComponentInfoFromProxy,)> {
    let out: CallResult<(ComponentInfoFromProxy,)> =
        ic_cdk::api::call::call(proxy, "get_component_info", ()).await;
    out
}

async fn get_registered_canister_in_db(
    db: Principal,
    target: Principal,
) -> CallResult<(Option<RegisteredCanisterInRegistry>,)> {
    let out: CallResult<(Option<RegisteredCanisterInRegistry>,)> =
        ic_cdk::api::call::call(db, "getRegisteredCanister", (target,)).await;
    out
}

#[query]
#[candid_method(query)]
fn get_last_metrics() -> Option<MetricsSnapshot> {
    METRICS.with(|mem| {
        let metrics = mem.borrow();
        if metrics.is_empty() {
            return None;
        }
        let last = metrics.last();
        last.cloned()
    })
}

#[query]
#[candid_method(query)]
fn get_metrics_interval_secs() -> Option<u64> {
    METRIC_TIMER_ID.with(|id| {
        if let Some((_, interval_secs)) = *id.borrow() {
            return Some(interval_secs);
        }
        None
    })
}

#[update]
#[candid_method(update)]
async fn start_metrics_timer(interval_secs: u64) {
    // only controllers can start the timer
    let res = canister_status(CanisterIdRecord {
        canister_id: ic_cdk::id(),
    })
    .await
    .unwrap()
    .0;
    assert!(
        res.settings.controllers.contains(&caller()),
        "Not permitted"
    );

    // clear the previous timer
    METRIC_TIMER_ID.with(|id| {
        if let Some((timer_id, _)) = *id.borrow() {
            ic_cdk_timers::clear_timer(timer_id);
        }
    });

    // execute
    let timer_id =
        ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(interval_secs), || {
            ic_cdk::spawn(async move { save_current_metrics().await });
        });
    METRIC_TIMER_ID.with(|id| *id.borrow_mut() = Some((timer_id, interval_secs)));
    save_current_metrics().await;
}

async fn save_current_metrics() {
    let res = canister_status(CanisterIdRecord {
        canister_id: ic_cdk::api::id(),
    })
    .await;
    if let Ok(status) = res {
        let cycles =
            u128::try_from(status.0.cycles.0).expect("Failed to convert cycles from Nat to u128");
        let timestamp = ic_cdk::api::time();
        ic_cdk::println!(
            "save_current_metrics: timestamp = {}, cycles = {}",
            timestamp,
            cycles
        );
        METRICS.with(|mem| {
            // save only the latest 1 metrics
            let mut metrics = mem.borrow_mut();
            if !metrics.is_empty() {
                metrics.remove(0);
            }
            metrics.push(MetricsSnapshot { timestamp, cycles });
        });
    }
}

#[update]
#[candid_method(update)]
async fn call_canister_status(canister_id: Principal) -> CanisterStatusResponse {
    // only controllers can start the timer
    let res = canister_status(CanisterIdRecord {
        canister_id: ic_cdk::id(),
    })
    .await
    .unwrap()
    .0;
    assert!(
        res.settings.controllers.contains(&caller()),
        "Not permitted"
    );

    let res = canister_status(CanisterIdRecord { canister_id }).await;
    res.unwrap().0
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

    let (state,): (UpgradeStableState,) =
        storage::stable_restore().expect("Failed to restore stable state");
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
