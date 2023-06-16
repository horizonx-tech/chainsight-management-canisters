import CA "mo:candb/CanisterActions";
import CanDB "mo:candb/CanDB";
import Entity "mo:candb/Entity";
import TimeStampedSk "TimeStampedSK";
import Canister "Canister";
import Principal "mo:base/Principal";
import Text "mo:base/Text";

module CanisterRepository {
    let delimiter = ":";
    type DB = actor {
        put : (opts : CanDB.PutOptions) -> async ();
        get : (opts : CanDB.GetOptions) -> async ?Entity.Entity;
    };

    public class Repository(_db : DB) {
        let db = _db;
        public let put : (Canister.Canister) -> async () = func(canister : Canister.Canister) : async () {
            await db.put({
                sk = sk(canister.principal);
                attributes = [];
            });
        };
        public let get : (principal : Principal) -> async ?Canister.Canister = func(principal : Principal) : async ?Canister.Canister {
            switch (await db.get({ sk = sk(principal) })) {
                case null { null };
                case (?canisterEntity) { ?unwrapCanister(canisterEntity) };
            };
        };

        func unwrapCanister(entity : Entity.Entity) : Canister.Canister {
            let { sk } = entity;
            let prefix = "Canister" # delimiter;
            let principal = Principal.fromText(Text.replace(sk, #text prefix, ""));
            {
                principal;
            };
        };
        func sk(principal : Principal) : Text.Text {
            return "Canister" #delimiter #Principal.toText(principal);
        };
    };

};
