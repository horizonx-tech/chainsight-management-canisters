import Time "mo:base/Time";
import Principal "mo:base/Principal";
module Types {
    type CallLog = {
        canister : Canister;
        calledBy : Canister;
        at : Time.Time;
    };
    type CalledLog = {
        canister : Canister;
        callTo : Canister;
        at : Time.Time;
    };
    type Canister = {
        principal : Principal;
    };
};
