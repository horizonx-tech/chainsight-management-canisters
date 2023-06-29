use std::ops::Sub;
use std::{cell::RefCell, ops::Add, str::FromStr};

use candid::{Nat, Principal};
use ic_cdk::{api::call::msg_cycles_accept128, caller, query, update};

use ic_stable_structures::{memory_manager::MemoryId, Cell, StableBTreeMap};
use ic_stable_structures::{
    memory_manager::{MemoryManager, VirtualMemory},
    DefaultMemoryImpl,
};
use types::types::{Balance, Index};
mod types;
use crate::types::types::Depositor;

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static MAP: RefCell<StableBTreeMap<Depositor, Balance, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
        )
    );
    static TOTAL_BALANCE: RefCell<Cell<Balance,Memory>> = RefCell::new(Cell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))), Balance::default()).unwrap());
    static CHAINSIGHT_CANISTER_ID : RefCell<Cell<String,Memory>> = RefCell::new(Cell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))), "".to_string()).unwrap());
    static INDEX: RefCell<Cell<Index,Memory>> = RefCell::new(Cell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3))), Index::default()).unwrap());
}

#[update]
async fn deposit() {
    let accepted = msg_cycles_accept128(u128::MAX);
    add_balance(caller(), Nat::from(accepted))
}

#[ic_cdk::init]
fn init(param: Vec<u8>) {
    set_chainsight_canister_id(Principal::from_slice(param.as_slice()))
}

fn set_chainsight_canister_id(principal: Principal) {
    CHAINSIGHT_CANISTER_ID.with(|m| {
        m.borrow_mut().set(principal.to_string()).unwrap();
    })
}

fn add_balance(principal: Principal, value: Nat) {
    let depositor = Depositor::from(principal);
    let added: Nat = Balance::from(value.clone()).div(index()).into();
    MAP.with(|m| {
        let balance: Nat = m.borrow().get(&depositor).unwrap_or_default().into();
        m.borrow_mut()
            .insert(depositor, Balance::from(balance.add(added.clone())));
    });
    add_total_balance(value, false);
}

fn add_total_balance(value: Nat, neg: bool) {
    TOTAL_BALANCE.with(|m| {
        let balance: Nat = m.borrow().get().into();
        let after = match neg {
            true => balance.sub(value),
            false => balance.add(value),
        };
        m.borrow_mut().set(after.into()).unwrap();
    })
}

#[query]
fn balance_of(principal: Principal) -> Nat {
    MAP.with(|m| {
        let balance = m.borrow().get(&principal.into()).unwrap_or_default();
        ic_cdk::println!("balance of {} is {:?}", principal, balance);
        let idx = index();
        ic_cdk::println!("index is {:?}", idx);
        balance.mul(idx).into()
    })
}
#[update]
fn consume(delta: Nat) {
    add_index(delta.clone(), true);
    add_total_balance(delta, true);
}

#[update]
fn supply(delta: Nat) {
    add_index(delta.clone(), false);
    add_total_balance(delta, false);
}

fn index() -> Index {
    INDEX.with(|m| m.borrow().get().clone())
}

#[query]
fn total_balance() -> Nat {
    TOTAL_BALANCE.with(|m| m.borrow().get().into())
}

#[update]
fn set_canister(principal: Principal) -> bool {
    let result = CHAINSIGHT_CANISTER_ID.with(|c| c.borrow_mut().set(principal.to_text()));
    match result {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn target_canister() -> Principal {
    CHAINSIGHT_CANISTER_ID.with(|c| Principal::from_str(c.borrow().get().as_str()).unwrap())
}

fn add_index(delta: Nat, neg: bool) {
    let idx = Index::percent(delta, total_balance());
    INDEX.with(|m| {
        let current = index();
        let after = match neg {
            true => current.sub(idx.into()),
            false => current.add(idx.into()),
        };
        m.borrow_mut().set(after).unwrap();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_index() {
        let delta = 10;
        add_total_balance(1_000_000_000_000u128.into(), false);
        add_index(delta.into(), true);
        add_total_balance(10.into(), true);
        assert_eq!(index(), Index::from(Balance::from(99_999_999_999)));
        assert_eq!(total_balance(), Nat::from(999_999_999_990u128));
    }
}
