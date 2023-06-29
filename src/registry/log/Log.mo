import Time "mo:base/Time";
import Principal "mo:base/Principal";
module Log {
    public type CallLog = Log;
    public type CalledLog = Log;
    public type Log = {
        canister : Principal;
        interactTo : Principal;
        at : Time.Time;
    };

    public let newCallLog : (Principal, Principal) -> CallLog = func(canister : Principal, interactTo : Principal) : CallLog = newLog(canister, interactTo);
    public let newCalledLog : (Principal, Principal) -> CalledLog = func(canister : Principal, interactTo : Principal) : CalledLog = newLog(canister, interactTo);
    func newLog(canister : Principal, interactTo : Principal) : Log = {
        canister = canister;
        interactTo = interactTo;
        at = Time.now();
    };
};
