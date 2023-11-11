use std::{
    borrow::Cow,
    ops::{Add, Div, Mul, Sub},
};

use candid::{CandidType, Decode, Encode, Nat, Principal};
use ic_stable_structures::{BoundedStorable, Storable};
use serde::Deserialize;

#[derive(CandidType, Deserialize, Default, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct Balance(u128);

impl From<u128> for Balance {
    fn from(val: u128) -> Self {
        Self(val)
    }
}
impl Into<u128> for Balance {
    fn into(self) -> u128 {
        self.0
    }
}

impl Balance {
    pub fn add(&self, bal: &Balance) -> Balance {
        self.0.clone().add(bal.0).into()
    }
    pub fn sub(&self, bal: &Balance) -> Balance {
        self.0.clone().sub(bal.0).into()
    }
}

#[derive(CandidType, Deserialize, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct Index(u128);

impl Index {
    pub fn add(&self, val: &Index) -> Index {
        self.0.clone().add(val.0).into()
    }
    pub fn sub(&self, val: &Index) -> Index {
        self.0.clone().sub(val.0).into()
    }
    pub fn share(&self, val: &Balance, total: &Balance) -> Index {
        if total.0 == 0 {
            return Index::from(val.0);
        }
        Index::from(val.0.mul(self.0).div(total.0))
    }
    pub fn to_balance(&self, total: &Index, supply: &Balance) -> Balance {
        if total.0 == 0 {
            return Balance::from(0);
        }
        if self > total {
            panic!("Invalid index: 'total' must be greater than or equals to 'self'");
        }
        Balance::from(self.0.mul(supply.0).div(total.0))
    }
}
impl Default for Index {
    fn default() -> Self {
        Self(0)
    }
}
impl Into<u128> for Index {
    fn into(self) -> u128 {
        self.0
    }
}
impl From<u128> for Index {
    fn from(val: u128) -> Self {
        Self(val)
    }
}

#[derive(CandidType, Deserialize, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct Depositor(Principal);
impl From<Principal> for Depositor {
    fn from(principal: Principal) -> Self {
        Self(principal)
    }
}

#[derive(CandidType, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct RefuelTarget {
    pub id: Principal,
    pub amount: u128,
    pub threshold: u128,
}

#[derive(CandidType, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ComponentMetricsSnapshot {
    pub timestamp: u64,
    pub cycles: u128,
}

#[derive(CandidType, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CycleBalance {
    pub id: Principal,
    pub amount: Nat,
}

impl Storable for Balance {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}
impl Storable for Index {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}
impl Storable for Depositor {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}
impl Storable for RefuelTarget {
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
}
impl Storable for ComponentMetricsSnapshot {
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
}
impl BoundedStorable for Depositor {
    const MAX_SIZE: u32 = 100;
    const IS_FIXED_SIZE: bool = false;
}
impl BoundedStorable for Balance {
    const MAX_SIZE: u32 = 100;
    const IS_FIXED_SIZE: bool = false;
}
impl BoundedStorable for Index {
    const MAX_SIZE: u32 = 100;
    const IS_FIXED_SIZE: bool = false;
}
impl BoundedStorable for RefuelTarget {
    const MAX_SIZE: u32 = 100;
    const IS_FIXED_SIZE: bool = false;
}
impl BoundedStorable for ComponentMetricsSnapshot {
    const MAX_SIZE: u32 = 100;
    const IS_FIXED_SIZE: bool = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

    #[test]
    fn test_balance_add() {
        assert_eq!(Balance::from(1).add(&Balance::from(1)), 2.into());
        assert_eq!(Balance::default().add(&Balance::from(1)), 1.into());
    }
    #[test]
    fn test_balance_sub() {
        assert_eq!(Balance::from(2).sub(&Balance::from(1)), 1.into());
        assert_eq!(Balance::from(1).sub(&Balance::from(1)), Balance::default());
    }
    #[test]
    fn test_index_add() {
        assert_eq!(Index::from(1).add(&Index::from(1)), 2.into());
        assert_eq!(Index::default().add(&Index::from(1)), 1.into());
    }
    #[test]
    fn test_index_sub() {
        assert_eq!(Index::from(2).sub(&Index::from(1)), 1.into());
        assert_eq!(Index::from(1).sub(&Index::from(1)), Index::default());
    }
    #[test]
    fn test_index_share() {
        assert_eq!(
            Index::from(100).share(&Balance::from(50), &Balance::from(100)),
            Index::from(50)
        );
        assert_eq!(
            Index::from(10).share(&Balance::from(50), &Balance::from(100)),
            Index::from(5)
        );
    }
    #[test]
    fn test_index_to_balance() {
        assert_eq!(
            Index::from(50).to_balance(&Index::from(100), &Balance::from(100)),
            Balance::from(50)
        );
        assert_eq!(
            Index::from(5).to_balance(&Index::from(10), &Balance::from(100)),
            Balance::from(50)
        );
        assert_eq!(
            Index::from(5).to_balance(&Index::from(10), &Balance::from(50)),
            Balance::from(25)
        );
    }
    #[test]
    fn test_index_to_balance_panic() {
        assert!(panic::catch_unwind(|| {
            Index::from(10).to_balance(&Index::from(5), &Balance::from(50))
        })
        .is_err());
    }

    #[test]
    fn test_refuel_setting_storable() {
        let setting = RefuelTarget {
            id: Principal::from_text("vvqfh-4aaaa-aaaao-a2mua-cai").unwrap(),
            threshold: 100,
            amount: 200,
        };
        assert_eq!(setting, RefuelTarget::from_bytes(setting.to_bytes()));
    }
}
