use std::{
    borrow::Cow,
    ops::{Add, Div, Mul, Sub},
};

use candid::{CandidType, Decode, Encode, Nat, Principal};
use ic_stable_structures::{BoundedStorable, Storable};
use serde::Deserialize;

#[derive(CandidType, Deserialize, Default, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct Balance(Nat);

impl Into<Nat> for Balance {
    fn into(self) -> Nat {
        self.0.into()
    }
}
impl Into<Nat> for &Balance {
    fn into(self) -> Nat {
        self.0.clone()
    }
}
impl From<Nat> for Balance {
    fn from(nat: Nat) -> Self {
        Self(nat)
    }
}
impl From<u128> for Balance {
    fn from(nat: u128) -> Self {
        Self(Nat::from(nat))
    }
}

impl Balance {
    pub fn mul(&self, idx: Index) -> Balance {
        let val = self.0.clone();
        let idx = idx.0 .0;
        val.mul(idx).div(Index::default().0 .0).into()
    }
    pub fn div(&self, idx: Index) -> Balance {
        let val = self.0.clone();
        let idx = idx.0 .0;
        val.mul(Index::default().0 .0).div(idx).into()
    }
}

#[derive(CandidType, Deserialize, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct Index(Balance);

impl Index {
    pub fn as_balance(&self) -> Balance {
        self.into()
    }
    pub fn mul(&self, b: Balance) -> Balance {
        let a: Nat = self.as_balance().into();
        let mul: Nat = b.0;
        a.mul(mul).div(Index::default().0 .0).into()
    }
    pub fn add(&self, n: Nat) -> Index {
        let a: Nat = self.as_balance().into();
        let b: Nat = n;
        let c: Balance = a.add(b).into();
        Index::from(c)
    }
    pub fn sub(&self, n: Nat) -> Index {
        let a: Nat = self.as_balance().into();
        let b: Nat = n;
        let c: Balance = a.sub(b).into();
        Index::from(c)
    }
    pub fn percent(a: Nat, b: Nat) -> Index {
        let diff: Balance = a.mul(Index::default().0 .0).div(b).into();
        Index::from(diff)
    }
}
impl Default for Index {
    fn default() -> Self {
        Self(Balance::from(Nat::from(100_000_000_000u128)))
    }
}
impl Into<Balance> for Index {
    fn into(self) -> Balance {
        self.0.into()
    }
}
impl Into<Balance> for &Index {
    fn into(self) -> Balance {
        self.0.clone().into()
    }
}
impl Into<Nat> for Index {
    fn into(self) -> Nat {
        self.0.into()
    }
}
impl From<Balance> for Index {
    fn from(balance: Balance) -> Self {
        Self(balance)
    }
}

#[derive(CandidType, Deserialize, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct Depositor(Principal);
impl From<Principal> for Depositor {
    fn from(principal: Principal) -> Self {
        Self(principal)
    }
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_index_as_balance() {
        let index = Index::default();
        let balance: Balance = index.as_balance();
        assert_eq!(balance, Balance::from(Nat::from(100_000_000_000u128)));
    }
    #[test]
    fn test_index_mul() {
        let index = Index::default();
        let balance: Balance = Balance::from(1);
        assert_eq!(index.mul(balance.clone()), balance);
    }
    #[test]
    fn test_index_add() {
        let index = Index::default();
        let expected: Index = Index::from(Balance::from(100_000_000_001u128));
        assert_eq!(index.add(1.into()), expected);
    }
    #[test]
    fn test_index_sub() {
        let index = Index::default();
        let expected: Index = Index::from(Balance::from(99_999_999_999u128));
        assert_eq!(index.sub(1.into()), expected);
    }
    #[test]
    fn test_index_percent() {
        let expected: Index = Index::from(Balance::from(50_000_000_000u128));
        assert_eq!(
            Index::percent(50_000_000_000u128.into(), 100_000_000_000u128.into()),
            expected
        );
    }
    #[test]
    fn test_balance_mul() {
        let balance = Balance::from(100_000_000_000u128);
        let expected: Balance = Balance::from(50_000_000_000u128); // 1/2
        assert_eq!(
            balance.mul(Index::from(Balance::from(50_000_000_000u128))),
            expected
        );
    }
    #[test]
    fn test_balance_div() {
        let balance = Balance::from(100_000_000_000u128);
        let expected: Balance = Balance::from(200_000_000_000u128); // 2
        assert_eq!(
            balance.div(Index::from(Balance::from(50_000_000_000u128))),
            expected
        );
    }
}
