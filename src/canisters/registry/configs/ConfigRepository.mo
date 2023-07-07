import CA "mo:candb/CanisterActions";
import CanDB "mo:candb/CanDB";
import Entity "mo:candb/Entity";
import TimeStampedSk "../db/TimeStampedSK";
import Principal "mo:base/Principal";
import Text "mo:base/Text";
import Config "Config";

module ConfigRepository {
    type DB = actor {
        put : (opts : CanDB.PutOptions) -> async ();
        get : (opts : CanDB.GetOptions) -> async ?Entity.Entity;
    };

    public class Repository(_db : DB) {
        let db = _db;
        public let put : (Config.Config) -> async () = func(config : Config.Config) : async () {
            await db.put({
                sk = config.id;
                attributes = [("value", #text(config.value))];
            });
        };
        public let get : (id : Text) -> async ?Config.Config = func(id : Text) : async ?Config.Config {
            switch (await db.get({ sk = id })) {
                case null { null };
                case (?entity) { ?unwrap(entity) };
            };
        };

        func unwrap(entity : Entity.Entity) : Config.Config {
            let { sk; attributes } = entity;
            let id = sk;
            let value = switch (Entity.getAttributeMapValueForKey(attributes, "value")) {
                case (?(#text(t))) { t };
                case (_) { "" };
            };
            { id = id; value = value };
        };
    };

};
