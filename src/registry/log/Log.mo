import Time "mo:base/Time";
import Principal "mo:base/Principal";
import Canister "../canister/Canister";
module Log {
    public type CallLog = Log;
    public type CalledLog = Log;
    public type Log = {
        canister : Canister.Canister;
        interactTo : Canister.Canister;
        at : Time.Time;
    };

    public let newCallLog : (Canister.Canister, Canister.Canister) -> CallLog = func(canister : Canister.Canister, interactTo : Canister.Canister) : CallLog = newLog(canister, interactTo);
    public let newCalledLog : (Canister.Canister, Canister.Canister) -> CalledLog = func(canister : Canister.Canister, interactTo : Canister.Canister) : CalledLog = newLog(canister, interactTo);
    func newLog(canister : Canister.Canister, interactTo : Canister.Canister) : Log = {
        canister = canister;
        interactTo = interactTo;
        at = Time.now();
    };
};
