import CA "mo:candb/CanisterActions";
import CanDB "mo:candb/CanDB";
import Entity "mo:candb/Entity";
import TimeStampedSk "../db/TimeStampedSK";
import Principal "mo:base/Principal";
import Text "mo:base/Text";
import Log "../log/Log";
import Iter "mo:base/Iter";
import Time "mo:base/Time";
import Option "mo:base/Option";
import Array "mo:base/Array";
import RBT "mo:stable-rbtree/StableRBTree";

module LogRepository {
    let delimiter = ":";
    type DB = actor {
        put : (opts : CanDB.PutOptions) -> async ();
        get : (opts : CanDB.GetOptions) -> async ?Entity.Entity;
        scan : (opts : CanDB.ScanOptions) -> async CanDB.ScanResult;
    };

    public class Repository(_db : DB) {
        let db = _db;
        public let putCallLog : (Log.CallLog) -> async () = func(log : Log.CallLog) : async () {
            await db.put({
                sk = TimeStampedSk.callLogSK(log.canister, log.at);
                attributes = [("interactTo", #text(Principal.toText(log.interactTo)))];
            });
        };
        public let put : (Log.CallLog) -> async () = func(log : Log.CallLog) : async () {
            await db.put({
                sk = TimeStampedSk.calledLogSK(log.canister, log.at);
                attributes = [("interactTo", #text(Principal.toText(log.interactTo)))];
            });
        };

        public func list(canister : Principal, from : Time.Time, to : ?Time.Time) : async ([Log.CallLog]) {
            let upperBound = switch (to) {
                case (?t) { TimeStampedSk.calledLogSK(canister, t) };
                case (null) {
                    TimeStampedSk.calledLogSK(canister, Time.now());
                };
            };
            let lowerBound = TimeStampedSk.calledLogSK(canister, from);
            let { entities; nextKey } = await db.scan({
                skLowerBound = lowerBound;
                skUpperBound = upperBound;
                limit = 10000;
                ascending = null;
            });
            Array.map<Entity.Entity, Log.CallLog>(entities, unwrap);
        };

        func unwrap(entity : Entity.Entity) : Log.Log {
            let { sk; attributes } = entity;
            let _sk = TimeStampedSk.fromSKText(sk);
            let interact = switch (Entity.getAttributeMapValueForKey(attributes, "interactTo")) {
                case (?(#text(t))) { Principal.fromText(t) };
                case (_) { Principal.fromText("") };
            };
            {
                canister = Principal.fromText(_sk.id);
                at = _sk.time;
                interactTo = interact;
            };
        };
    };

};
