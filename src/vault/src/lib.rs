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

fn add_balance(principal: Principal, value: Nat) {
    let depositor = Depositor::from(principal);
    let added: Nat = Balance::from(value).div(index()).into();
    MAP.with(|m| {
        let balance: Nat = m.borrow().get(&depositor).unwrap_or_default().into();
        m.borrow_mut()
            .insert(depositor, Balance::from(balance.add(added.clone())));
    });
    TOTAL_BALANCE.with(|m| {
        let balance: Nat = m.borrow().get().into();
        m.borrow_mut()
            .set(Balance::from(balance.add(added)))
            .unwrap();
    })
}

#[query]
fn balance_of(principal: Principal) -> Nat {
    MAP.with(|m| {
        let balance = m
            .borrow()
            .get(&principal.into())
            .map(Balance::into)
            .unwrap_or_default();
        let idx = index();
        idx.mul(balance).into()
    })
}

fn consume(delta: Nat) {
    let total_balance_after = total_balance().sub(delta);
    _add_index(total_balance_after, true)
}

fn supply(delta: Nat) {
    let total_balance_after = total_balance().add(delta);
    _add_index(total_balance_after, false)
}

fn _add_index(after: Nat, neg: bool) {
    let diff = Index::percent(after, total_balance());
    add_index(diff.as_balance().into(), neg);
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

fn add_index(val: Nat, neg: bool) {
    INDEX.with(|m| match neg {
        true => m.borrow_mut().set(m.borrow().get().add(val)).unwrap(),
        false => m.borrow_mut().set(m.borrow().get().sub(val)).unwrap(),
    });
}
