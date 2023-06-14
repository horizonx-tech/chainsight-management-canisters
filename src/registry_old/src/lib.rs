use std::cell::RefCell;
mod types;
use candid::Principal;
use common::CanisterRegisterInput;
use ic_cdk::update;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    writer::Writer,
    DefaultMemoryImpl, StableBTreeMap,
};
use types::{ChainsightCanister, ID};

type Memory = VirtualMemory<DefaultMemoryImpl>;

const PROXY_MEMORY_ID: MemoryId = MemoryId::new(0);
const OWNER_MEMORY_ID: MemoryId = MemoryId::new(1);
const CANISTER_MEMORY_ID: MemoryId = MemoryId::new(2);

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static CANISTERS: RefCell<StableBTreeMap<ID, ChainsightCanister, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(CANISTER_MEMORY_ID)),
        )
    );
    static PROXY: RefCell<ID> = RefCell::new(ID::new(Principal::anonymous()));
    static OWNER: RefCell<ID> = RefCell::new(ID::new(Principal::anonymous()));
}
// modifiers
fn only_proxy() {
    _only(PROXY.with(|p| p.borrow().clone()).to_principal())
}

fn _only(principal: Principal) {
    if principal == Principal::anonymous() {
        ic_cdk::trap("Principal is not set");
    }
    let calleer = ic_cdk::caller();
    if calleer != principal {
        ic_cdk::trap("unauthorized");
    }
}

fn only_owner() {
    _only(OWNER.with(|o| o.borrow().clone()).to_principal())
}

// upgrades

#[ic_cdk::init]
fn init() {
    _set_proxy(ic_cdk::caller());
    ic_cdk::println!("Init with caller: {}", ic_cdk::caller().to_text());
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    _pre_upgrade(
        PROXY_MEMORY_ID,
        PROXY.with(|p| p.borrow().to_principal().to_text().into_bytes()),
    );
    _pre_upgrade(
        OWNER_MEMORY_ID,
        OWNER.with(|o| o.borrow().to_principal().to_text().into_bytes()),
    );
}

fn _pre_upgrade(mem_id: MemoryId, data: Vec<u8>) {
    let len = data.len();
    let mut memory = MEMORY_MANAGER.with(|m| m.borrow().get(mem_id));
    let mut writer = Writer::new(&mut memory, 0);
    writer.write(&len.to_le_bytes()).unwrap();
    writer.write(&data).unwrap();
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let proxy = _post_upgrade(PROXY_MEMORY_ID);
    let owner = _post_upgrade(OWNER_MEMORY_ID);
    PROXY.with(|p| *p.borrow_mut() = ID::new(proxy));
    OWNER.with(|o| *o.borrow_mut() = ID::new(owner));
}

fn _post_upgrade(mem_id: MemoryId) -> Principal {
    let memory = MEMORY_MANAGER.with(|m| m.borrow().get(mem_id));
    let mut state_len_bytes = [0; 4];
    ic_stable_structures::Memory::read(&memory, 0, &mut state_len_bytes);
    let state_len = u32::from_le_bytes(state_len_bytes) as usize;
    let mut state_bytes = vec![0; state_len];
    ic_stable_structures::Memory::read(&memory, 4, &mut state_bytes);
    let txt = String::from_utf8(state_bytes).unwrap();
    Principal::from_text(&txt).unwrap()
}

// management functions

#[update]
fn set_proxy(principal: Principal) {
    only_owner();
    _set_proxy(principal)
}

fn _set_proxy(principal: Principal) {
    PROXY.with(|p| *p.borrow_mut() = ID::new(principal));
}

#[update]
fn set_owner(principal: Principal) {
    only_owner();
    _set_owner(principal)
}

fn _set_owner(principal: Principal) {
    OWNER.with(|o| *o.borrow_mut() = ID::new(principal));
}

// canister functions

#[update]
fn register(input: CanisterRegisterInput) {
    only_proxy();
    let id = ID::new(input.principal);
    let canister = ChainsightCanister::new(ID::new(input.craeted_by));
    CANISTERS.with(|c| c.borrow_mut().insert(id, canister));
}

#[update]
fn unregister(principal: Principal) {
    only_proxy();
    let id = ID::new(principal);
    CANISTERS.with(|c| c.borrow_mut().remove(&id));
}
