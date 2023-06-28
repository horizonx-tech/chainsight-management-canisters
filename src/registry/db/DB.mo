import CA "mo:candb/CanisterActions";
import CanDB "mo:candb/CanDB";
import Entity "mo:candb/Entity";
import TimeStampedSk "../db/TimeStampedSK";
import Canister "../canister/Canister";
import Principal "mo:base/Principal";
import Text "mo:base/Text";
import Debug "mo:base/Debug";

shared ({ caller = owner }) actor class DB({
    // the primary key of this canister
    partitionKey : Text;
    // the scaling options that determine when to auto-scale out this canister storage partition
    scalingOptions : CanDB.ScalingOptions;
    // (optional) allows the developer to specify additional owners (i.e. for allowing admin or backfill access to specific endpoints)
    owners : ?[Principal];
}) {
    let delimiter = ":";
    /// @required (may wrap, but must be present in some form in the canister)
    stable let db = CanDB.init({
        pk = partitionKey;
        scalingOptions = scalingOptions;
        btreeOrder = null;
    });

    /// @recommended (not required) public API
    public query func getPK() : async Text { db.pk };

    /// @required public API (Do not delete or change)
    public query func skExists(sk : Text) : async Bool {
        CanDB.skExists(db, sk);
    };

    /// @required public API (Do not delete or change)
    public shared ({ caller = caller }) func transferCycles() : async () {
        if (caller == owner) {
            return await CA.transferCycles(caller);
        };
    };

    public shared ({ caller = caller }) func put(opts : CanDB.PutOptions) : async () {
        assert (caller == owner);
        Debug.print("opts.sk");
        Debug.print(opts.sk);
        await* CanDB.put(
            db,
            opts,
        );
    };

    public shared ({ caller = caller }) func scan(opts : CanDB.ScanOptions) : async CanDB.ScanResult {
        assert (caller == owner);
        CanDB.scan(
            db,
            opts,
        );
    };

    public shared ({ caller = caller }) func get(opts : CanDB.GetOptions) : async ?Entity.Entity {
        return CanDB.get(
            db,
            opts,
        );
    };

};
