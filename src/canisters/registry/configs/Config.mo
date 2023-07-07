import Time "mo:base/Time";
import Principal "mo:base/Principal";
import Text "mo:base/Text";
module Configs {
    public type Config = {
        id : Text;
        value : Text;
    };
    func newCanister(id : Text, value : Text) : Config {
        return { id = id; value = value };
    };
    public func newProxy(principal : Principal) : Config {
        return newCanister(
            "proxy",
            Principal.toText(principal),
        );
    };
};
