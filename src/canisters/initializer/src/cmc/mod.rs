pub mod types;
use candid::Principal;
use types::Service;

pub fn cmc() -> Service {
    // https://github.com/dfinity/cdk-rs/blob/main/library/ic-ledger-types/src/lib.rs#L873-L877
    Service(Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").unwrap())
}
