use std::cell::RefCell;

use candid::{candid_method, CandidType, Int, Principal};
use ic_cdk::{
    api::call::{call_with_payment128, CallResult, RejectionCode},
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
    static VAULT: RefCell<Principal>  = RefCell::new(Principal::anonymous());
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
fn init(registry: Principal, vault: Principal, target: Principal, db: Principal) {
    REGISTRY.with(|r| *r.borrow_mut() = registry);
    VAULT.with(|v| *v.borrow_mut() = vault);
    TARGET.with(|t| *t.borrow_mut() = target);
    DB.with(|d| *d.borrow_mut() = db);
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
#[update]
fn current_commission() -> u128 {
    _current_commission()
}

fn _current_commission() -> u128 {
    // TODO
    1
}
fn _vault() -> Principal {
    VAULT.with(|vault| vault.borrow().clone())
}

#[query]
fn current_commission_read() -> u128 {
    _current_commission()
}

#[update]
async fn invoke_proxy_call(
    dst_proxy: Principal,
    method: String,
    args: Vec<u8>,
) -> CallResult<(Vec<u8>,)> {
    let _commission: CallResult<(u128,)> =
        ic_cdk::api::call::call(dst_proxy, "current_commission", ()).await;
    if _commission.is_err() {
        ic_cdk::trap(_commission.err().unwrap().1.as_str());
    }
    let commission = _commission.unwrap().0;
    let approve_result: CallResult<()> =
        ic_cdk::api::call::call(_vault(), "approve", (dst_proxy, commission)).await;
    if approve_result.is_err() {
        ic_cdk::trap(approve_result.err().unwrap().1.as_str());
    }
    let call_result: CallResult<(Vec<u8>,)> =
        ic_cdk::api::call::call(dst_proxy, "proxy_call", (_vault(), method.as_str(), args)).await;
    if call_result.is_err() {
        ic_cdk::trap(call_result.err().unwrap().1.as_str());
    }
    call_result
}

async fn _proxy_call(vault: Principal, method: String, args: Vec<u8>) -> CallResult<(Vec<u8>,)> {
    let commission = _current_commission();
    let withdraw_result: CallResult<()> =
        ic_cdk::api::call::call(vault, "withdraw", (commission,)).await;
    if withdraw_result.is_err() {
        ic_cdk::println!("withdraw_result: {:?}", withdraw_result);
        ic_cdk::trap(withdraw_result.err().unwrap().1.as_str())
    }
    let deposit_result: CallResult<()> =
        call_with_payment128(_vault(), "deposit_commission", (), commission).await;
    if deposit_result.is_err() {
        ic_cdk::println!("deposit_result: {:?}", deposit_result);
        ic_cdk::trap(deposit_result.err().unwrap().1.as_str());
    }
    ic_cdk::println!("proxy call method: {}", method.as_str());
    let result: CallResult<(Vec<u8>,)> =
        ic_cdk::api::call::call(_target(), method.as_str(), (args,)).await;
    if result.is_err() {
        ic_cdk::println!("Error: {:?}", result);
    }
    result
}

#[update]
async fn proxy_call(vault: Principal, method: String, args: Vec<u8>) -> CallResult<(Vec<u8>,)> {
    let result = _proxy_call(vault, method, args).await;
    _put_call_log(ic_cdk::caller()).await;
    result
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
fn set_registry(id: Principal) {
    REGISTRY.with(|registry| {
        *registry.borrow_mut() = id;
    });
}
