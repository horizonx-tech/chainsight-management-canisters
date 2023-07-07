import CA "mo:candb/CanisterActions";
import CanDB "mo:candb/CanDB";
import Entity "mo:candb/Entity";
import TimeStampedSk "../db/TimeStampedSK";
import Canister "Canister";
import Principal "mo:base/Principal";
import Text "mo:base/Text";
import Array "mo:base/Array";

module CanisterRepository {
    let delimiter = ":";
    type DB = actor {
        put : (opts : CanDB.PutOptions) -> async ();
        get : (opts : CanDB.GetOptions) -> async ?Entity.Entity;
        scan : (opts : CanDB.ScanOptions) -> async CanDB.ScanResult;
    };

    public class Repository(_db : DB) {
        let db = _db;
        public let put : (Canister.Canister) -> async () = func(canister : Canister.Canister) : async () {
            await db.put({
                sk = sk(canister.principal);
                attributes = [("vault", #text(Principal.toText(canister.vault)))];
            });
        };
        public let get : (principal : Principal) -> async ?Canister.Canister = func(principal : Principal) : async ?Canister.Canister {
            switch (await db.get({ sk = sk(principal) })) {
                case null { null };
                case (?canisterEntity) { ?unwrapCanister(canisterEntity) };
            };
        };
        public let list : (lower : Text, upper : Text) -> async [Canister.Canister] = func(lower : Text, upper : Text) : async [Canister.Canister] {
            let { entities } = await db.scan({
                skLowerBound = "Canister" # delimiter # lower;
                skUpperBound = "Canister" # delimiter # upper;
                limit = 10000;
                ascending = null;
            });
            Array.map<Entity.Entity, Canister.Canister>(entities, unwrapCanister);
        };

        func unwrapCanister(entity : Entity.Entity) : Canister.Canister {
            let { sk; attributes } = entity;
            let prefix = "Canister" # delimiter;
            let principal = Principal.fromText(Text.replace(sk, #text prefix, ""));
            let vault = switch (Entity.getAttributeMapValueForKey(attributes, "vault")) {
                case (?(#text(t))) { Principal.fromText(t) };
                case (_) { Principal.fromText("") };
            };
            {
                principal;
                vault;
            };
        };
        func sk(principal : Principal) : Text.Text {
            return "Canister" #delimiter #Principal.toText(principal);
        };
    };

};
