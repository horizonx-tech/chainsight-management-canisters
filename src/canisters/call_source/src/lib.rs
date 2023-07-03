use std::{cell::RefCell, str::FromStr};

use candid::Principal;
use common::{ExampleCallArgs, ExampleCallResult, ExampleInnerStruct};
use ic_cdk::update;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    writer::Writer,
    DefaultMemoryImpl,
};
use rpc::{
    call_provider::{self, CallProvider},
    caller::Caller,
    message::Message,
};

type Memory = VirtualMemory<DefaultMemoryImpl>;
const PROXY_MEMORY_ID: MemoryId = MemoryId::new(0);
const DEST_MEMORY_ID: MemoryId = MemoryId::new(1);

thread_local! {
// The memory manager is used for simulating multiple memories. Given a `MemoryId` it can
// return a memory that can be used by stable structures.
static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
    RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static PROXY: RefCell<Principal> = RefCell::new(Principal::anonymous());
    static DEST: RefCell<Principal> = RefCell::new(Principal::anonymous());
}

#[update]
async fn example_call() -> ExampleCallResult {
    let example_input = ExampleCallArgs {
        bytes: [1, 2, 3].to_vec(),
        num: 1,
        text: "hello".to_string(),
        sample_struct: ExampleInnerStruct {
            txt: "world".to_string(),
        },
    };
    let msg = Message::new(example_input, dest(), "example_call").unwrap();
    let result = CallProvider::new(proxy()).call(msg).await;
    match result {
        Ok(result) => result.reply::<ExampleCallResult>().unwrap(),
        Err(err) => panic!("{}", err.1),
    }
}

// management functions
#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    _pre_upgrade(
        PROXY_MEMORY_ID,
        PROXY.with(|p| p.borrow().to_text().into_bytes()),
    );
}

#[update]
fn set_proxy(principal: String) {
    PROXY.with(|p| *p.borrow_mut() = Principal::from_str(principal.as_str()).unwrap());
}
#[update]
fn set_dest(principal: String) {
    DEST.with(|p| *p.borrow_mut() = Principal::from_str(principal.as_str()).unwrap());
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
    PROXY.with(|p| *p.borrow_mut() = proxy);
    let dest = _post_upgrade(DEST_MEMORY_ID);
    DEST.with(|p| *p.borrow_mut() = dest);
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

fn proxy() -> Principal {
    PROXY.with(|p| *p.borrow())
}
fn dest() -> Principal {
    DEST.with(|p| *p.borrow())
}
