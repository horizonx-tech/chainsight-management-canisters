use candid::{candid_method, Principal};
use ic_cdk::{
    api::{
        call::msg_cycles_accept128,
        canister_balance128,
        management_canister::{
            main::{canister_status, deposit_cycles},
            provisional::CanisterIdRecord,
        },
    },
    caller, query, update,
};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    Cell, DefaultMemoryImpl, StableBTreeMap,
};
use std::{cell::RefCell, str::FromStr, time::Duration};
use types::types::{Balance, Index, RefuelTarget};
mod types;
use crate::types::types::Depositor;

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static SHARE_MAP: RefCell<StableBTreeMap<Depositor, Index, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
        )
    );
    static TOTAL_SUPPLY: RefCell<Cell<Balance,Memory>> = RefCell::new(Cell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))), Balance::default()).unwrap());
    static CHAINSIGHT_CANISTER_ID : RefCell<Cell<String,Memory>> = RefCell::new(Cell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))), "".to_string()).unwrap());
    static INDEX: RefCell<Cell<Index,Memory>> = RefCell::new(Cell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3))), Index::default()).unwrap());
    static REFUEL_TARGETS: RefCell<ic_stable_structures::Vec<RefuelTarget,Memory>> = RefCell::new(
        ic_stable_structures::Vec::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4)))).unwrap()
    );
}

#[ic_cdk::init]
fn init(
    chainsight_caniseter: Principal,
    deployer: Principal,
    initial_supply: Balance,
    refueling_interval_secs: u64,
) {
    set_chainsight_canister_id(chainsight_caniseter);
    increase_index(&initial_supply, deployer);
    start_refueling(refueling_interval_secs);
}

#[update]
#[candid_method(update)]
fn supply(principal: Option<Principal>) {
    increase_index(
        &msg_cycles_accept128(u128::MAX).into(),
        principal.unwrap_or(caller()),
    );
}

#[update]
#[candid_method(update)]
async fn withdraw(delta: Balance) {
    let principal = caller();
    if withdrawable_of(principal).lt(&delta) {
        panic!("Not enough withdrawable balance");
    }
    decrease_index(&delta, principal);
    deposit_cycles(
        CanisterIdRecord {
            canister_id: principal,
        },
        delta.into(),
    )
    .await
    .unwrap();
}

#[query]
#[candid_method(query)]
fn total_supply() -> Balance {
    TOTAL_SUPPLY.with(
        |m: &RefCell<Cell<Balance, VirtualMemory<std::rc::Rc<RefCell<Vec<u8>>>>>>| {
            m.borrow().get().clone()
        },
    )
}

#[query]
#[candid_method(query)]
fn index() -> Index {
    INDEX.with(|m| m.borrow().get().clone())
}

#[query]
#[candid_method(query)]
fn balance_of(principal: Principal) -> Balance {
    SHARE_MAP.with(|m| {
        let share = m.borrow().get(&principal.into()).unwrap_or_default();
        share.to_balance(&index(), &total_supply()).into()
    })
}

#[query]
#[candid_method(query)]
fn withdrawable_of(principal: Principal) -> Balance {
    salvage_stray_cycles();
    SHARE_MAP.with(|m| {
        let share = m.borrow().get(&principal.into()).unwrap_or_default();
        share.to_balance(&index(), &Balance::from(canister_balance128()))
    })
}

#[query]
#[candid_method(query)]
fn share_of(principal: Principal) -> Index {
    SHARE_MAP.with(|m| m.borrow().get(&principal.into()).unwrap_or_default())
}

fn increase_index(delta: &Balance, principal: Principal) {
    add_share(principal, delta, false);
    add_index(delta, false);
    add_total_supply(delta, false);
}

fn decrease_index(delta: &Balance, principal: Principal) {
    add_share(principal, delta, true);
    add_index(delta, true);
    add_total_supply(delta, true);
}

fn add_total_supply(value: &Balance, neg: bool) {
    TOTAL_SUPPLY.with(|m| {
        let balance: Balance = m.borrow().get().clone();
        let after = match neg {
            true => balance.sub(value),
            false => balance.add(value),
        };
        m.borrow_mut().set(after.into()).unwrap();
    })
}

fn add_index(delta: &Balance, neg: bool) {
    let current = index();
    let idx = &current.share(delta, &total_supply());
    INDEX.with(|m| {
        let after = match neg {
            true => current.sub(idx),
            false => current.add(idx),
        };
        m.borrow_mut().set(after).unwrap();
    });
}

fn add_share(principal: Principal, delta: &Balance, neg: bool) {
    SHARE_MAP.with(|m| {
        let share = m.borrow().get(&principal.into()).unwrap_or_default();
        let idx_delta = &index().share(delta, &total_supply());
        let after = match neg {
            true => share.sub(idx_delta),
            false => share.add(idx_delta),
        };
        m.borrow_mut().insert(principal.into(), after);
    })
}

fn salvage_stray_cycles() {
    let actual_balance: Balance = canister_balance128().into();
    if actual_balance > total_supply() {
        TOTAL_SUPPLY.with(|m| {
            m.borrow_mut().set(actual_balance).unwrap();
        })
    }
}

#[update]
#[candid_method(update)]
fn receive_revenue() {
    let accepted = msg_cycles_accept128(u128::MAX);
    if accepted == 0 {
        panic!("No cycles received")
    }
    add_total_supply(&Balance::from(accepted), false);
}

#[update]
#[candid_method(update)]
async fn refuel() {
    for target in get_refuel_targets() {
        let res = canister_status(CanisterIdRecord {
            canister_id: target.id,
        })
        .await;
        if let Ok(status) = res {
            let balance = status.0.cycles;
            if balance > target.threashold {
                continue;
            }
        }
        // TODO handle error except out of cycles
        deposit_cycles(
            CanisterIdRecord {
                canister_id: target.id,
            },
            target.value,
        )
        .await
        .unwrap();
    }
}

#[update]
#[candid_method(update)]
async fn put_refuel_target(target: RefuelTarget) {
    let res = canister_status(CanisterIdRecord {
        canister_id: ic_cdk::id(),
    })
    .await
    .unwrap()
    .0;
    if !res.settings.controllers.contains(&target.id) {
        panic!("Not permitted")
    }
    _put_refuel_target(target);
}

fn _put_refuel_target(target: RefuelTarget) {
    let position = REFUEL_TARGETS.with(|m| m.borrow().iter().position(|s| s.id == target.id));
    if let Some(i) = position {
        REFUEL_TARGETS.with(|m| {
            m.borrow_mut().set(i as u64, &target);
        })
    } else {
        REFUEL_TARGETS.with(|m| {
            m.borrow_mut().push(&target).unwrap();
        })
    }
}

#[query]
#[candid_method(query)]
fn get_refuel_targets() -> Vec<RefuelTarget> {
    REFUEL_TARGETS.with(|m| m.borrow().iter().map(|s| s.clone()).collect::<Vec<_>>())
}

#[query]
#[candid_method(query)]
fn target_canister() -> Principal {
    CHAINSIGHT_CANISTER_ID.with(|c| Principal::from_str(c.borrow().get().as_str()).unwrap())
}

#[update]
#[candid_method(update)]
fn set_canister(principal: Principal) -> bool {
    let result = CHAINSIGHT_CANISTER_ID.with(|c| c.borrow_mut().set(principal.to_text()));
    match result {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn set_chainsight_canister_id(principal: Principal) {
    CHAINSIGHT_CANISTER_ID.with(|m| {
        m.borrow_mut().set(principal.to_string()).unwrap();
    })
}

fn start_refueling(interval_secs: u64) {
    ic_cdk_timers::set_timer_interval(Duration::from_secs(interval_secs), || {
        ic_cdk::spawn(refuel())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index() {
        let depositor1 = Principal::from_text("vvqfh-4aaaa-aaaao-a2mua-cai").unwrap();

        // initial supply
        let initial = 1_000;
        increase_index(&initial.into(), depositor1);
        assert_eq!(index(), Index::from(initial));
        assert_eq!(share_of(depositor1), index());
        assert_eq!(total_supply(), Balance::from(initial));
        assert_eq!(balance_of(depositor1), total_supply());

        // withdraw
        let delta = 400;
        decrease_index(&delta.into(), depositor1);
        assert_eq!(index(), Index::from(600));
        assert_eq!(share_of(depositor1), index());
        assert_eq!(total_supply(), Balance::from(600));
        assert_eq!(balance_of(depositor1), total_supply());

        // receive revenue
        let delta = 300;
        add_total_supply(&delta.into(), false);
        assert_eq!(index(), Index::from(600));
        assert_eq!(share_of(depositor1), index());
        assert_eq!(total_supply(), Balance::from(900));
        assert_eq!(balance_of(depositor1), total_supply());

        let depositor2 = Principal::from_text("vsrdt-ryaaa-aaaao-a2muq-cai").unwrap();
        // supply
        let delta = 300;
        increase_index(&delta.into(), depositor2);
        assert_eq!(index(), Index::from(800));
        assert_eq!(share_of(depositor1), Index::from(600));
        assert_eq!(share_of(depositor2), Index::from(200));
        assert_eq!(total_supply(), Balance::from(1200));
        assert_eq!(balance_of(depositor1), Balance::from(900));
        assert_eq!(balance_of(depositor2), Balance::from(300));

        // withdraw
        let delta = 150;
        decrease_index(&delta.into(), depositor2);
        assert_eq!(index(), Index::from(700));
        assert_eq!(share_of(depositor1), Index::from(600));
        assert_eq!(share_of(depositor2), Index::from(100));
        assert_eq!(total_supply(), Balance::from(1050));
        assert_eq!(balance_of(depositor1), Balance::from(900));
        assert_eq!(balance_of(depositor2), Balance::from(150));
    }

    #[test]
    fn test_put_refuel_target() {
        let mut target1 = RefuelTarget {
            id: Principal::from_text("vvqfh-4aaaa-aaaao-a2mua-cai").unwrap(),
            threashold: 100,
            value: 200,
        };
        _put_refuel_target(target1);
        assert_eq!(get_refuel_targets()[0], target1);
        assert_eq!(get_refuel_targets().len(), 1);

        let target2 = RefuelTarget {
            id: Principal::from_text("vsrdt-ryaaa-aaaao-a2muq-cai").unwrap(),
            threashold: 1000,
            value: 2000,
        };
        _put_refuel_target(target2);
        assert_eq!(get_refuel_targets()[1], target2);
        assert_eq!(get_refuel_targets().len(), 2);

        target1.value = 300;
        _put_refuel_target(target1);
        assert_eq!(get_refuel_targets()[0].value, 300);
        assert_eq!(get_refuel_targets().len(), 2);
    }
}
