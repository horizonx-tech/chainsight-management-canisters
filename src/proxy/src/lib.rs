use std::{cell::RefCell, str::FromStr};

use candid::{CandidType, Int, Principal};
use ic_cdk::{
    api::call::{CallResult, RejectionCode},
    update,
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
    static REGISTRY: RefCell<String> = RefCell::new(String::new());
}

fn registry() -> Principal {
    REGISTRY.with(|registry| Principal::from_str(&registry.borrow()).unwrap())
}

#[update]
async fn list_logs(principal: Principal, from: Int, to: Int) -> Vec<CallLog> {
    let call_result: CallResult<(Vec<CallLog>,)> =
        ic_cdk::api::call::call(registry(), "listLogsOf", (principal, from, to)).await;
    match call_result {
        Ok((logs,)) => logs,
        Err(err) => {
            ic_cdk::println!("Error: {:?}", err);
            vec![]
        }
    }
}

#[update]
async fn proxy_call(
    from: Principal,
    id: Principal,
    method: String,
    args: Vec<u8>,
) -> CallResult<(Vec<u8>,)> {
    if !canister_exists(from).await {
        ic_cdk::println!("Unknown canster: {:?}", id.to_string());
        return Err((
            RejectionCode::DestinationInvalid,
            format!("Unknown canister: {}", id.to_string()),
        ));
    }
    if !canister_exists(id).await {
        ic_cdk::println!("Unknown canster: {:?}", id.to_string());
        return Err((
            RejectionCode::DestinationInvalid,
            format!("Unknown canister: {}", id.to_string()),
        ));
    }
    let result: CallResult<(Vec<u8>,)> =
        ic_cdk::api::call::call(id, method.as_str(), (args,)).await;
    if result.is_err() {
        ic_cdk::println!("Error: {:?}", result);
    }
    _put_call_log(from, id).await;
    result
}

async fn canister_exists(id: Principal) -> bool {
    let result: CallResult<(bool,)> = ic_cdk::api::call::call(registry(), "exists", (id,)).await;
    //match result {
    //    Ok(result) => result.0,
    //    Err(e) => {
    //        ic_cdk::println!("Error: {:?}", e);
    //        false
    //    }
    //}
    true
}
#[update]
async fn put_call_log(call_to: Principal) {
    _put_call_log(ic_cdk::caller(), call_to).await;
}

async fn _put_call_log(call_from: Principal, call_to: Principal) {
    let result: CallResult<()> =
        ic_cdk::api::call::call(registry(), "putLog", (call_from, call_to)).await;
    match result {
        Err(e) => ic_cdk::println!("Error: {:?}", e),
        _ => (),
    }
}

#[update]
fn set_registry(id: String) {
    Principal::from_str(&id).unwrap();
    REGISTRY.with(|registry| {
        *registry.borrow_mut() = id;
    });
}

#[update]
async fn register(principal: String) {
    let _: CallResult<()> = ic_cdk::api::call::call(
        registry(),
        "registerCanister",
        (Principal::from_str(principal.as_str()).unwrap(),),
    )
    .await;
}
