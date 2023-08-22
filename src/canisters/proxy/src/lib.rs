use std::cell::RefCell;

use candid::{CandidType, Int, Principal};
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

thread_local! {
    static REGISTRY: RefCell<Principal> = RefCell::new(Principal::anonymous());
    static TARGET: RefCell<Principal> = RefCell::new(Principal::anonymous());
    static DB: RefCell<Principal> = RefCell::new(Principal::anonymous());
    static KNOWN_CANISTERS: RefCell<Vec<Principal>> = RefCell::new(vec![]);
}

#[query]
fn db() -> Principal {
    _db()
}
fn _registry() -> Principal {
    REGISTRY.with(|registry| registry.borrow().clone())
}

#[query]
fn registry() -> Principal {
    _registry()
}
fn _target() -> Principal {
    TARGET.with(|target| target.borrow().clone())
}

#[query]
fn target() -> Principal {
    _target()
}
fn _db() -> Principal {
    DB.with(|db| db.borrow().clone())
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

async fn _proxy_call(method: String, args: Vec<u8>) -> CallResult<(Vec<u8>,)> {
    if !canister_exists(ic_cdk::caller()).await {
        ic_cdk::println!("Unknown canster: {:?}", ic_cdk::caller().to_string());
        return Err((
            RejectionCode::CanisterReject,
            format!("Unknown canister: {}", ic_cdk::caller().to_string()),
        ));
    }
    let result: CallResult<(Vec<u8>,)> =
        ic_cdk::api::call::call(_target(), method.as_str(), (args,)).await;
    if result.is_err() {
        ic_cdk::println!("Error: {:?}", result);
    }
    result
}

#[update]
async fn proxy_call(method: String, args: Vec<u8>) -> CallResult<(Vec<u8>,)> {
    let result = _proxy_call(method, args).await;
    _put_call_log().await;
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

async fn _put_call_log() {
    let result: CallResult<()> =
        ic_cdk::api::call::call(_db(), "putLog", (ic_cdk::caller(), _target())).await;
    match result {
        Err(e) => ic_cdk::println!("Error: {:?}", e),
        _ => (),
    }
}

#[update]
fn set_registry(id: Principal) {
    REGISTRY.with(|registry| {
        *registry.borrow_mut() = id;
    });
}
