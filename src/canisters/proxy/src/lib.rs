use std::cell::RefCell;

use candid::{candid_method, CandidType, Int, Principal};
use ic_cdk::{
    api::call::{CallResult, RejectionCode},
    query, update,
};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CallLog {
    canister: Canister,
    #[serde(rename = "interactTo")]
    interact_to: Canister,
    at: Int,
}

#[derive(CandidType, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Canister {
    principal: Principal,
}

#[derive(Clone, Debug, Default, candid::CandidType, candid::Deserialize, serde::Serialize)]
pub struct IndexingConfig {
    pub task_interval_secs: u32,
    pub method: String,
    pub args: Vec<u8>,
}

#[derive(Clone, Debug, Default, candid::CandidType, candid::Deserialize, serde::Serialize)]
pub struct ExecutionResult {
    pub is_succeeded: bool,
    pub timestamp: u64,
    pub error: Option<Error>,
}

#[derive(Clone, Debug, Default, candid::CandidType, candid::Deserialize, serde::Serialize)]
pub struct Error {
    pub message: String,
    // pub backtrace: String,
}

thread_local! {
    static REGISTRY: RefCell<Principal> = RefCell::new(Principal::anonymous());
    static TARGET: RefCell<Principal> = RefCell::new(Principal::anonymous());
    static DB: RefCell<Principal> = RefCell::new(Principal::anonymous());
    static KNOWN_CANISTERS: RefCell<Vec<Principal>> = RefCell::new(vec![]);
    static INDEXING_CONFIG: std::cell::RefCell<IndexingConfig> = std::cell::RefCell::new(IndexingConfig::default());
    static LAST_SUCCEEDED: std::cell::RefCell<u64> = std::cell::RefCell::new(0);
    static LAST_EXECUTION_RESULT: std::cell::RefCell<ExecutionResult> = std::cell::RefCell::new(ExecutionResult::default());
    static NEXT_SCHEDULE: std::cell::RefCell<u64> = std::cell::RefCell::new(0);
}

#[query]
#[candid_method(query)]
fn target() -> Principal {
    _target()
}

fn _target() -> Principal {
    TARGET.with(|target| target.borrow().clone())
}

#[query]
#[candid_method(query)]
fn db() -> Principal {
    _db()
}

fn _db() -> Principal {
    DB.with(|db| db.borrow().clone())
}

#[query]
#[candid_method(query)]
fn registry() -> Principal {
    _registry()
}

fn _registry() -> Principal {
    REGISTRY.with(|registry| registry.borrow().clone())
}


#[ic_cdk::init]
fn init(registry: Principal, target: Principal, db: Principal) {
    REGISTRY.with(|r| {
        *r.borrow_mut() = registry;
    });
    TARGET.with(|t| {
        *t.borrow_mut() = target;
    });
    DB.with(|d| {
        *d.borrow_mut() = db;
    });
}

#[update]
#[candid_method(update)]
async fn list_logs(from: Int, to: Int) -> Vec<CallLog> {
    let call_result: CallResult<(Vec<CallLog>,)> =
        ic_cdk::api::call::call(_registry(), "listLogsOf", (_target(), from, to)).await;
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
    _put_call_log(caller).await;
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

async fn canister_exists(id: Principal) -> bool {
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
    REGISTRY.with(|registry| {
        *registry.borrow_mut() = id;
    });
}

#[query]
#[candid_method(query)]
fn last_succeeded() -> u64 {
    LAST_SUCCEEDED.with(|x| *x.borrow())
}

#[query]
#[candid_method(query)]
fn last_execution_result() -> ExecutionResult {
    LAST_EXECUTION_RESULT.with(|x| x.borrow().clone())
}

#[query]
#[candid_method(query)]
fn next_schedule() -> u64 {
    NEXT_SCHEDULE.with(|f| *f.borrow())
}

fn set_next_schedule(time: u64) {
    NEXT_SCHEDULE.with(|f| *f.borrow_mut() = time);
}

#[query]
#[candid_method(query)]
fn get_indexing_config() -> IndexingConfig {
    INDEXING_CONFIG.with(|f| f.borrow().clone())
}

fn set_indexing_config(config: IndexingConfig) {
    INDEXING_CONFIG.with(|f| *f.borrow_mut() = config);
}

#[update]
#[candid_method(update)]
pub fn start_indexing(task_interval_secs: u32, delay_secs: u32, method: String, args: Vec<u8>) {
    if ic_cdk::caller() != _target() {
        panic!("Not permitted")
    }
    if next_schedule() != 0 {
        panic!("Already started")
    }
    let current_time_sec = (ic_cdk::api::time() / (1000 * 1000000)) as u32;
    let round_timestamp = |ts: u32, unit: u32| ts / unit * unit;
    let delay =
        round_timestamp(current_time_sec, task_interval_secs) + task_interval_secs + delay_secs
            - current_time_sec;
    set_indexing_config(IndexingConfig {
        task_interval_secs,
        method,
        args,
    });
    ic_cdk_timers::set_timer(std::time::Duration::from_secs(delay as u64), move || {
        ic_cdk_timers::set_timer_interval(
            std::time::Duration::from_secs(task_interval_secs as u64),
            || {
                ic_cdk::spawn(async move { index().await });
            },
        );
    });
    set_next_schedule((current_time_sec + delay + get_indexing_config().task_interval_secs) as u64);
}

async fn index() {
    let config = get_indexing_config();
    let current_time_sec = (ic_cdk::api::time() / (1000 * 1000000)) as u32;
    set_next_schedule((current_time_sec + config.task_interval_secs) as u64);

    let result: CallResult<(Vec<u8>,)> =
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
        LAST_SUCCEEDED.with(|x| *x.borrow_mut() = current_time_sec);
    }
    LAST_EXECUTION_RESULT.with(|x| {
        *x.borrow_mut() = ExecutionResult {
            is_succeeded: error.is_none(),
            timestamp: current_time_sec,
            error,
        }
    });
}
