import Time "mo:base/Time";
import Principal "mo:base/Principal";
module Canister {
    public type Canister = {
        principal : Principal;
        vault : Principal;
    };
    public let newCanister : (Principal, Principal) -> Canister = func(principal : Principal, vault : Principal) : Canister = {
        principal = principal;
        vault = vault;
    };
};
