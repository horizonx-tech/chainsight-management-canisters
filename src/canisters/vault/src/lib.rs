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
use types::types::{
    Balance, ComponentMetricsSnapshot, CycleBalance, Index, PrincipalStorable, RefuelTarget,
};
mod types;

type Memory = VirtualMemory<DefaultMemoryImpl>;

const MONITROING_INTERVAL_SECS: u64 = 3600;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static SHARE_MAP: RefCell<StableBTreeMap<PrincipalStorable, Index, Memory>> = RefCell::new(
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
    static CUMULATIVE_REFUELED: RefCell<StableBTreeMap<PrincipalStorable, u128, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5))),
        )
    );
    static COMPONENT_METRICS_SNAPSHOT: std::cell::RefCell<Vec<ComponentMetricsSnapshot>> = std::cell::RefCell::new(Vec::new());
}

#[ic_cdk::init]
async fn init(
    chainsight_caniseter: Principal,
    deployer: Principal,
    initial_supply: Balance,
    refueling_interval_secs: u64,
    refuel_targets: Vec<RefuelTarget>,
    refuel_targets_inital_supply: Vec<(Principal, u128)>,
) {
    set_chainsight_canister_id(chainsight_caniseter);
    increase_index(&initial_supply, deployer);
    start_refueling(refueling_interval_secs);
    refuel_targets.iter().for_each(_put_refuel_target);
    refuel_targets_inital_supply
        .iter()
        .for_each(|(id, amount)| {
            record_cumulative_refueled(*id, *amount);
        });
    start_monitoring_component_metrics(MONITROING_INTERVAL_SECS).await;
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
    TOTAL_SUPPLY.with(|m| m.borrow().get().clone())
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
    ic_cdk::println!("Start refueling...");
    for target in get_refuel_targets() {
        let res = canister_status(CanisterIdRecord {
            canister_id: target.id,
        })
        .await;
        if let Ok(status) = res {
            let balance = status.0.cycles;
            ic_cdk::println!(
                "[{}] balance: {}",
                target.id.to_string(),
                balance.to_string(),
            );
            if balance > target.threshold {
                ic_cdk::println!(
                    "[{}] skip refueling: threshold={}",
                    target.id.to_string(),
                    target.threshold.to_string(),
                );
                continue;
            }
        }
        // TODO handle error except out of cycles
        deposit_cycles(
            CanisterIdRecord {
                canister_id: target.id,
            },
            target.amount,
        )
        .await
        .unwrap();
        record_cumulative_refueled(target.id, target.amount);
        ic_cdk::println!(
            "[{}] refueled: {} ",
            target.id.to_string(),
            target.amount.to_string(),
        );
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
    if !res.settings.controllers.contains(&caller()) {
        panic!("Not permitted")
    }
    _put_refuel_target(&target);
}

fn _put_refuel_target(target: &RefuelTarget) {
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

#[update]
#[candid_method(update)]
async fn get_cycle_balances() -> Vec<CycleBalance> {
    let mut balances: Vec<CycleBalance> = vec![];
    let targets = get_refuel_targets();
    let res = futures::future::join_all(targets.iter().map(|t| async {
        let status = canister_status(CanisterIdRecord { canister_id: t.id })
            .await
            .unwrap();
        CycleBalance {
            id: t.id,
            amount: status.0.cycles,
        }
    }));

    let id = ic_cdk::id();
    let status = canister_status(CanisterIdRecord { canister_id: id })
        .await
        .unwrap();
    balances.push(CycleBalance {
        id,
        amount: status.0.cycles,
    });

    res.await.into_iter().for_each(|b| balances.push(b));

    balances
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

#[ic_cdk::query]
#[candid::candid_method(query)]
pub fn metric() -> ComponentMetricsSnapshot {
    COMPONENT_METRICS_SNAPSHOT.with(|m| m.borrow().iter().last().unwrap().clone())
}

#[ic_cdk::query]
#[candid::candid_method(query)]
pub fn metrics(n: usize) -> Vec<ComponentMetricsSnapshot> {
    COMPONENT_METRICS_SNAPSHOT
        .with(|m| m.borrow().iter().rev().take(n).cloned().collect::<Vec<_>>())
}

async fn start_monitoring_component_metrics(interval_secs: u64) {
    let round_timestamp = |ts: u64, unit: u64| ts / unit * unit;
    let current_time_sec = (ic_cdk::api::time() / (1000 * 1000000)) as u64;
    let delay = round_timestamp(current_time_sec, interval_secs) + interval_secs - current_time_sec;

    ic_cdk_timers::set_timer(std::time::Duration::from_secs(delay), move || {
        ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(interval_secs), || {
            ic_cdk::spawn(monitor_component_metrics());
        });
    });

    ic_cdk_timers::set_timer(std::time::Duration::from_secs(0), move || {
        ic_cdk::spawn(monitor_component_metrics());
    });
}

async fn monitor_component_metrics() {
    let timestamp = ic_cdk::api::time();
    let balances = get_cycle_balances().await;
    let datum = ComponentMetricsSnapshot {
        timestamp,
        cycles: balances
            .iter()
            .map(|b| u128::try_from(b.amount.0.clone()).unwrap())
            .sum(),
    };
    ic_cdk::println!("monitoring: {:?}", datum.clone());
    add_component_metrics_snapshot(datum);
}

fn add_component_metrics_snapshot(datum: ComponentMetricsSnapshot) {
    COMPONENT_METRICS_SNAPSHOT.with(|m| {
        m.borrow_mut().push(datum);
    })
}

#[query]
#[candid_method(query)]
fn get_cumulative_refueled(target: Principal) -> u128 {
    CUMULATIVE_REFUELED.with(|m| m.borrow().get(&target.into()).unwrap_or_default().clone())
}

#[query]
#[candid_method(query)]
fn get_cumulative_refueled_all() -> Vec<(Principal, u128)> {
    CUMULATIVE_REFUELED.with(|m| {
        m.borrow()
            .iter()
            .map(|(k, v)| (k.0.clone(), v.clone()))
            .collect()
    })
}

fn record_cumulative_refueled(target: Principal, amount: u128) {
    CUMULATIVE_REFUELED.with(|m| {
        let balance = m.borrow().get(&target.into()).unwrap_or_default();
        let after = balance + amount;
        m.borrow_mut().insert(target.into(), after);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    candid::export_service!();

    #[test]
    fn generate_candid() {
        std::fs::write("vault.did", __export_service()).unwrap();
    }

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
            threshold: 100,
            amount: 200,
        };
        _put_refuel_target(&target1);
        assert_eq!(get_refuel_targets()[0], target1);
        assert_eq!(get_refuel_targets().len(), 1);

        let target2 = RefuelTarget {
            id: Principal::from_text("vsrdt-ryaaa-aaaao-a2muq-cai").unwrap(),
            threshold: 1000,
            amount: 2000,
        };
        _put_refuel_target(&target2);
        assert_eq!(get_refuel_targets()[1], target2);
        assert_eq!(get_refuel_targets().len(), 2);

        target1.amount = 300;
        _put_refuel_target(&target1);
        assert_eq!(get_refuel_targets()[0].amount, 300);
        assert_eq!(get_refuel_targets().len(), 2);
    }
}
