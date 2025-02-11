use std::{borrow::Cow, cell::RefCell};

use candid::{candid_method, CandidType, Decode, Encode, Int, Principal};
use ic_cdk::{
    api::call::{CallResult, RejectionCode}, post_upgrade, query, update
};
use ic_cdk_timers::TimerId;
use ic_stable_structures::{memory_manager::{MemoryId, MemoryManager, VirtualMemory}, DefaultMemoryImpl};
use serde::{Deserialize, Serialize};

type MemoryType = VirtualMemory<DefaultMemoryImpl>;

#[derive(CandidType, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CallLog {
    canister: Principal,
    #[serde(rename = "interactTo")]
    interact_to: Principal,
    at: Int,
}

#[derive(Clone, Debug, Default, candid::CandidType, candid::Deserialize, serde::Serialize)]
pub struct IndexingConfig {
    pub task_interval_secs: u32,
    pub method: String,
    pub args: Vec<u8>,
    pub delay_secs: Option<u32>,
    pub is_rounded_start_time: Option<bool>,
}
impl ic_stable_structures::Storable for IndexingConfig {
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
}

#[derive(Clone, Debug, Default, candid::CandidType, candid::Deserialize, serde::Serialize)]
pub struct ExecutionResult {
    pub is_succeeded: bool,
    pub timestamp: u64,
    pub error: Option<Error>,
}
impl ic_stable_structures::Storable for ExecutionResult {
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
}

#[derive(Clone, Debug, Default, candid::CandidType, candid::Deserialize, serde::Serialize)]
pub struct Error {
    pub message: String,
    // pub backtrace: String,
}

#[derive(Clone, Debug, candid::CandidType, candid::Deserialize, serde::Serialize)]
pub struct ComponentInfo {
    pub target: Principal,
    pub vault: Principal,
    pub db: Principal,
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    // sidecar
    static TARGET: RefCell<ic_stable_structures::StableCell<String, MemoryType>> = RefCell::new(
        ic_stable_structures::StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))),
            String::new(),
        ).unwrap()
    );
    static DB: RefCell<ic_stable_structures::StableCell<String, MemoryType>> = RefCell::new(
        ic_stable_structures::StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))),
            String::new(),
        ).unwrap()
    );
    static VAULT: RefCell<ic_stable_structures::StableCell<String, MemoryType>> = RefCell::new(
        ic_stable_structures::StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3))),
            String::new(),
        ).unwrap()
    );

    // manager
    static INITIALIZER: RefCell<ic_stable_structures::StableCell<String, MemoryType>> = RefCell::new(
        ic_stable_structures::StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4))),
            String::new(),
        ).unwrap()
    );
    static REGISTRY: RefCell<ic_stable_structures::StableCell<String, MemoryType>> = RefCell::new(
        ic_stable_structures::StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5))),
            String::new(),
        ).unwrap()
    );

    // static KNOWN_CANISTERS: RefCell<Vec<Principal>> = RefCell::new(vec![]);
    static INDEXING_CONFIG: RefCell<ic_stable_structures::StableCell<IndexingConfig, MemoryType>> = RefCell::new(
        ic_stable_structures::StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(6))),
            IndexingConfig::default(),
         ).unwrap()
    );
    static LAST_SUCCEEDED: RefCell<ic_stable_structures::StableCell<u64, MemoryType>> = RefCell::new(
        ic_stable_structures::StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(7))),
            0,
         ).unwrap()
    );
    static LAST_EXECUTION_RESULT: RefCell<ic_stable_structures::StableCell<ExecutionResult, MemoryType>> = RefCell::new(
        ic_stable_structures::StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(8))),
            ExecutionResult::default(),
         ).unwrap()
    );
    static NEXT_SCHEDULE: RefCell<ic_stable_structures::StableCell<u64, MemoryType>> = RefCell::new(
        ic_stable_structures::StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(9))),
            0,
         ).unwrap()
    );

    // NOTE: TimerId cannot be stored in stable memory
    // https://github.com/dfinity/cdk-rs/issues/392
    static TIMER_ID: RefCell<Option<TimerId>> = RefCell::new(None);
}

#[query]
#[candid_method(query)]
fn get_component_info() -> ComponentInfo {
    ComponentInfo {
        target: _target(),
        vault: _vault(),
        db: _db(),
    }
}

#[query]
#[candid_method(query)]
fn target() -> Principal {
    _target()
}
fn _target() -> Principal {
    let res = TARGET.with(|target| target.borrow().get().clone());
    Principal::from_text(&res).unwrap()

}
fn _set_target(id: Principal) {
    let res = TARGET.with(|target| target.borrow_mut().set(id.to_text()));
    res.unwrap();
}

#[query]
#[candid_method(query)]
fn db() -> Principal {
    _db()
}
fn _db() -> Principal {
    let res = DB.with(|db| db.borrow().get().clone());
    Principal::from_text(&res).unwrap()
}
fn _set_db(id: Principal) {
    let res = DB.with(|db| db.borrow_mut().set(id.to_text()));
    res.unwrap();
}

#[query]
#[candid_method(query)]
fn vault() -> Principal {
    _vault()
}
fn _vault() -> Principal {
    let res = VAULT.with(|db| db.borrow().get().clone());
    Principal::from_text(&res).unwrap()
}
fn _set_vault(id: Principal) {
    let res = VAULT.with(|db| db.borrow_mut().set(id.to_text()));
    res.unwrap();
}

#[query]
#[candid_method(query)]
fn registry() -> Principal {
    _registry()
}
fn _registry() -> Principal {
    let res = REGISTRY.with(|registry| registry.borrow().get().clone());
    Principal::from_text(&res).unwrap()
}

#[query]
#[candid_method(query)]
fn initializer() -> Principal {
    _initializer()
}
fn _initializer() -> Principal {
    let res = INITIALIZER.with(|db| db.borrow().get().clone());
    Principal::from_text(&res).unwrap()
}
fn _set_initializer(id: Principal) {
    let res = INITIALIZER.with(|db| db.borrow_mut().set(id.to_text()));
    res.unwrap();
}

fn _timer_id() -> Option<TimerId> {
    TIMER_ID.with(|state| *state.borrow())
}
fn _set_timer_id(id: TimerId) {
    TIMER_ID.with(|state: &RefCell<Option<TimerId>>| *state.borrow_mut() = Some(id));
}
fn _reset_timer_id() {
    TIMER_ID.with(|state: &RefCell<Option<TimerId>>| *state.borrow_mut() = None);
}

#[ic_cdk::init]
fn init(registry: Principal, target: Principal, db: Principal, vault: Principal) {
    _set_target(target);
    _set_db(db);
    _set_vault(vault);
    set_registry(registry);
    _set_initializer(ic_cdk::caller()); // NOTE: Generated by initializer
}

#[update]
#[candid_method(update)]
async fn list_logs(target: Principal, from: Int, to: Int) -> Vec<CallLog> {
    let call_result: CallResult<(Vec<CallLog>,)> =
        ic_cdk::api::call::call(_db(), "listLogsOf", (target, from, to)).await;
    match call_result {
        Ok((logs,)) => logs,
        Err(err) => {
            ic_cdk::println!("Error: {:?}", err);
            vec![]
        }
    }
}

#[update]
#[candid_method(update)]
async fn proxy_call(method: String, args: Vec<u8>) -> CallResult<(Vec<u8>,)> {
    let caller = ic_cdk::caller();
    let result = _proxy_call(caller, method, args).await;
    // _put_call_log(caller).await;
    result
}

async fn _proxy_call(caller: Principal, method: String, args: Vec<u8>) -> CallResult<(Vec<u8>,)> {
    if !canister_exists(caller).await {
        ic_cdk::println!("Unknown canster: {:?}", caller.to_string());
        return Err((
            RejectionCode::CanisterReject,
            format!("Unknown canister: {}", caller.to_string()),
        ));
    }
    ic_cdk::println!("proxy call method: {}", method.as_str());
    let result: CallResult<(Vec<u8>,)> =
        ic_cdk::api::call::call(_target(), method.as_str(), (args,)).await;
    if result.is_err() {
        ic_cdk::println!("Error: {:?}", result);
    }
    result
}

async fn canister_exists(_id: Principal) -> bool {
    // TODO: payment
    true
    //let known = KNOWN_CANISTERS.with(|canisters| canisters.borrow().contains(&id));
    //if known {
    //    return true;
    //}
    //// TODO: This line can be a single-point-of-failure.
    //let result: CallResult<(bool,)> = ic_cdk::api::call::call(_registry(), "exists", (id,)).await;
    //match result {
    //    Ok((exists,)) => match exists {
    //        true => {
    //            KNOWN_CANISTERS.with(|canisters| canisters.borrow_mut().push(id));
    //            true
    //        }
    //        false => false,
    //    },
    //    Err(err) => {
    //        ic_cdk::println!("Error: {:?}", err);
    //        false
    //    }
    //}
}

async fn _put_call_log(caller: Principal) {
    let result: CallResult<()> =
        ic_cdk::api::call::call(_db(), "putLog", (caller, _target())).await;
    match result {
        Err(e) => ic_cdk::println!("Error: {:?}", e),
        _ => (),
    }
}

#[update]
#[candid_method(update)]
fn set_registry(id: Principal) {
    let res = REGISTRY.with(|registry| registry.borrow_mut().set(id.to_text()));
    res.unwrap();
}

#[query]
#[candid_method(query)]
fn last_succeeded() -> u64 {
    LAST_SUCCEEDED.with(|x| x.borrow().get().clone())
}

fn set_last_succeeded(v: u64) {
    let res = LAST_SUCCEEDED.with(|x| x.borrow_mut().set(v));
    res.unwrap();
}

#[query]
#[candid_method(query)]
fn last_execution_result() -> ExecutionResult {
    LAST_EXECUTION_RESULT.with(|x| x.borrow().get().clone())
}

fn set_last_execution_result(v: ExecutionResult) {
    let res = LAST_EXECUTION_RESULT.with(|x| x.borrow_mut().set(v));
    res.unwrap();
}

#[query]
#[candid_method(query)]
fn next_schedule() -> u64 {
    NEXT_SCHEDULE.with(|f| f.borrow().get().clone())
}

fn set_next_schedule(time: u64) {
    let res = NEXT_SCHEDULE.with(|f| f.borrow_mut().set(time));
    res.unwrap();
}

#[query]
#[candid_method(query)]
fn get_indexing_config() -> IndexingConfig {
    INDEXING_CONFIG.with(|f| f.borrow().get().clone())
}

fn set_indexing_config(config: IndexingConfig) {
    let res = INDEXING_CONFIG.with(|f| f.borrow_mut().set(config));
    res.unwrap();
}

#[update]
#[candid_method(update)]
pub fn start_indexing(task_interval_secs: u32, delay_secs: u32, method: String, args: Vec<u8>) {
    start_indexing_with_is_rounded(task_interval_secs, delay_secs, false, method, args);
}
// NOTE: `start_indexing` is kept for backward compatibility, `is_rounded_start_time` is added to the interface
//       Integrate with `start_indexing` when destructive changes are possible
#[update]
#[candid_method(update)]
pub fn start_indexing_with_is_rounded(task_interval_secs: u32, delay_secs: u32, is_rounded_start_time: bool, method: String, args: Vec<u8>) {
    assert!(ic_cdk::caller() == _target(), "Not permitted");
    assert!(next_schedule() == 0, "Already started");

    let indexing_config = IndexingConfig {
        task_interval_secs,
        method,
        args,
        delay_secs: Some(delay_secs),
        is_rounded_start_time: Some(is_rounded_start_time),
    };
    start_indexing_internal(indexing_config.clone());
    set_indexing_config(indexing_config);
}

fn start_indexing_internal(indexing_config: IndexingConfig) {
    let current_time_sec = (ic_cdk::api::time() / (1000 * 1000000)) as u32;
    let IndexingConfig {
        task_interval_secs,
        delay_secs,
        is_rounded_start_time,
        ..
    } = indexing_config;
    let delay = if is_rounded_start_time.is_some() {
        calculate_delay_secs_from_current_secs(current_time_sec, task_interval_secs, delay_secs.unwrap_or_default())
    } else {
        delay_secs.unwrap_or_default()
    };

    if delay > 0 {
        let timer_id = ic_cdk_timers::set_timer(std::time::Duration::from_secs(delay as u64), move || {
            let timer_id = ic_cdk_timers::set_timer_interval(
                std::time::Duration::from_secs(task_interval_secs as u64),
                || {
                    ic_cdk::spawn(async move { index().await });
                },
            );
            ic_cdk::spawn(async move { index().await }); // First execution after waiting for delay secs
            _set_timer_id(timer_id); // Save to overwrite TimerId by set_timer
        });
        _set_timer_id(timer_id);
        set_next_schedule((current_time_sec + delay) as u64);
    } else {
        ic_cdk::spawn(async move { index().await }); // If there is no delay, the program is executed immediately.
        let timer_id = ic_cdk_timers::set_timer_interval(
            std::time::Duration::from_secs(task_interval_secs as u64),
            || {
                ic_cdk::spawn(async move { index().await });
            },
        );
        _set_timer_id(timer_id);
    };
}

fn calculate_delay_secs_from_current_secs(current: u32, interval: u32, delay_secs: u32) -> u32 {
    let round_timestamp = |ts: u32, unit: u32| ts / unit * unit;
    round_timestamp(current, interval) + interval + delay_secs
            - current
}

async fn index() {
    let config = get_indexing_config();
    let current_time_sec = (ic_cdk::api::time() / (1000 * 1000000)) as u32;
    set_next_schedule((current_time_sec + config.task_interval_secs) as u64);

    let result: CallResult<(Option<Vec<u8>>,)> =
        ic_cdk::api::call::call(_target(), config.method.as_str(), (config.args,)).await;
    if result.is_ok() {
        update_last_execution_result(None);
    } else {
        update_last_execution_result(Some(Error {
            message: format!("{:?}", result),
        }));
    }
}

fn update_last_execution_result(error: Option<Error>) {
    let current_time_sec = (ic_cdk::api::time() / (1000 * 1000000)) as u64;
    if error.is_none() {
        set_last_succeeded(current_time_sec);
    }
    set_last_execution_result(ExecutionResult {
        is_succeeded: error.is_none(),
        timestamp: current_time_sec,
        error,
    });
}

#[update]
#[candid_method(update)]
async fn request_upgrades_to_registry() {
    if !ic_cdk::api::is_controller(&ic_cdk::caller()) {
        ic_cdk::trap("Not permitted");
    }

    let res: CallResult<((),)> = ic_cdk::api::call::call(_initializer(), "upgrade_proxies", ()).await;
    res.expect("Failed to call 'upgrade_proxies' to Initializer");
}

/// Restart indexing task
/// NOTE: Intended to be used when restarting after cycles are exhausted
///       Duplicate timer run if canister is stopped/restarted normally
/// ref: https://internetcomputer.org/docs/current/references/ic-interface-spec#global-timer
#[update]
#[candid_method(update)]
async fn restart_indexing() {
    let indexing_config = get_indexing_config();
    assert!(indexing_config.task_interval_secs > 0, "indexing_config is not yet set");

    if !ic_cdk::api::is_controller(&ic_cdk::caller())
        || ic_cdk::api::time() / 1_000_000_000
            > next_schedule() + (indexing_config.task_interval_secs as u64 * 2)
    {
        ic_cdk::trap("Not permitted");
    }

    if let Some(timer_id) = _timer_id() {
        ic_cdk_timers::clear_timer(timer_id);
        _reset_timer_id();
    }

    start_indexing_internal(indexing_config);
}

#[post_upgrade]
fn post_upgrade() {
    let config = get_indexing_config();
    if config.task_interval_secs > 0 {
        // If the timer was already started, set the timer again at the time of upgrade.
        start_indexing_internal(IndexingConfig {
            delay_secs: Some(1), // inter-canister call cannot be executed in init/post_upgrade
            ..config
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    candid::export_service!();

    #[test]
    fn generate_candid() {
        std::fs::write("proxy.did", __export_service()).unwrap();
    }

    #[test]
    fn test_calculate_delay_secs_from_current_secs() {
        let secs_20200101_000000 = 946684800;
        assert_eq!(
            calculate_delay_secs_from_current_secs(
                secs_20200101_000000 + 15,
                60,
                0
            ),
            45
        );
        assert_eq!(
            calculate_delay_secs_from_current_secs(
                secs_20200101_000000 + 15,
                90,
                0
            ),
            75
        );
        assert_eq!(
            calculate_delay_secs_from_current_secs(
                secs_20200101_000000 + 10 * 60,
                60 * 60,
                30
            ),
            50 * 60 + 30
        );
    }
}
