import Time "mo:base/Time";
import Principal "mo:base/Principal";
module Canister {
    public type Canister = {
        principal : Principal;
    };
    public let newCanister : (Principal) -> Canister = func(principal : Principal) : Canister = {
        principal = principal;
    };
};
