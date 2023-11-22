import Cycles "mo:base/ExperimentalCycles";
import Debug "mo:base/Debug";
import Error "mo:base/Error";
import Principal "mo:base/Principal";
import Text "mo:base/Text";

import CA "mo:candb/CanisterActions";
import CanisterMap "mo:candb/CanisterMap";
import Utils "mo:candb/Utils";
import DB "./db/DB";
import Canister "canister/Canister";
import CanDB "mo:candb/CanDB";
import Entity "mo:candb/Entity";
import CanisterRepository "canister/CanisterRepository";
import Array "mo:base/Array";
import Log "log/Log";
import LogRepository "log/LogRepository";
import Time "mo:base/Time";
import Buffer "mo:stable-buffer/StableBuffer";

shared ({ caller = owner }) actor class RegistryCanister() = this {
    type DB = actor {
        put : shared (opts : CanDB.PutOptions) -> async ();
        get : shared (opts : CanDB.GetOptions) -> async (?Entity.Entity);
        scan : shared (opts : CanDB.ScanOptions) -> async (CanDB.ScanResult);
    };
    type CanisterRepositoryIFace = {
        put : (canister : Canister.Canister) -> async ();
        get : (principal : Principal) -> async (?Canister.Canister);
        list : (lower : Text, upper : Text) -> async ([Canister.Canister]);
    };
    type LogRepositoryIFace = {
        put : Log.CallLog -> async ();
        list : (canister : Principal, from : Time.Time, to : ?Time.Time) -> async ([Log.CallLog]);
    };

    func listLogRepositories() : [LogRepositoryIFace] {
        Array.map<Text, LogRepositoryIFace>(
            getCanisterIdsIfExists("Logs"),
            func(canisterId) {
                logRepository(canisterId);
            },
        );
    };

    func logRepository(canisterId : Text) : LogRepositoryIFace {
        return LogRepository.Repository(db(canisterId));
    };

    func canisterRepository(canisterId : Text) : CanisterRepositoryIFace {
        return CanisterRepository.Repository(db(canisterId));
    };

    func db(canisterId : Text) : DB {
        actor (canisterId) : DB;
    };

    func listCanisterRepositories() : [CanisterRepositoryIFace] {
        Array.map<Text, CanisterRepositoryIFace>(
            getCanisterIdsIfExists("Canisters"),
            func(canisterId) {
                canisterRepository(canisterId);
            },
        );
    };

    public shared func registerCanister(principal : Principal, vault : Principal) : async () {
        switch (putDestination("Canisters")) {
            case null {
                Debug.trap("No canister registry found");
            };
            case (?dest) {
                await canisterRepository(dest).put(Canister.newCanister(principal, vault));
            };
        };
    };

    public shared func putLog(caller : Principal, callTo : Principal) : async () {
        await listLogRepositories()[0].put(Log.newCallLog(caller, callTo));
    };

    public shared func listLogsOf(principal : Principal, from : Time.Time, to : Time.Time) : async ([Log.CallLog]) {
        await listLogRepositories()[0].list(principal, from, ?to);
    };

    func putDestination(pk : Text) : ?Text {
        let canisterIds = getCanisterIdsIfExists(pk);
        let length = canisterIds.size();
        if (length == 0) {
            return null;
        };
        return ?canisterIds[length - 1];
    };

    public shared func exists(principal : Principal) : async Bool {
        for (repo in listCanisterRepositories().vals()) {
            if ((await repo.get(principal)) != null) {
                return true;
            };
        };
        return false;
    };

    public shared func scanCanisters() : async [Canister.Canister] {
        let res = Buffer.init<Canister.Canister>();
        for (repo in listCanisterRepositories().vals()) {
            let canisters = await repo.list("0", "zzzzz-zzzzz-zzzzz-zzzzz-zzz");
            Buffer.append(
                res,
                Buffer.fromArray<Canister.Canister>(canisters),
            );
        };
        return Buffer.toArray(res);
    };

    public shared (msg) func init() : async [?Text] {
        assert (owner == msg.caller);
        ([
            await createServiceCanister("Canisters"),
            await createServiceCanister("Logs"),
        ]);
    };

    /// @required stable variable (Do not delete or change)
    ///
    /// Holds the CanisterMap of PK -> CanisterIdList
    stable var pkToCanisterMap = CanisterMap.init();

    /// @required API (Do not delete or change)
    ///
    /// Get all canisters for an specific PK
    ///
    /// This method is called often by the candb-client query & update methods.
    public shared query ({ caller = caller }) func getCanistersByPK(pk : Text) : async [Text] {
        getCanisterIdsIfExists(pk);
    };

    /// @required function (Do not delete or change)
    ///
    /// Helper method acting as an interface for returning an empty array if no canisters
    /// exist for the given PK
    func getCanisterIdsIfExists(pk : Text) : [Text] {
        switch (CanisterMap.get(pkToCanisterMap, pk)) {
            case null { [] };
            case (?canisterIdsBuffer) { Buffer.toArray(canisterIdsBuffer) };
        };
    };

    public shared (msg) func autoScaleServiceCanister(pk : Text) : async Text {
        assert (owner == msg.caller);
        // Auto-Scaling Authorization - if the request to auto-scale the partition is not coming from an existing canister in the partition, reject it
        if (Utils.callingCanisterOwnsPK(owner, pkToCanisterMap, pk)) {
            Debug.print("creating an additional canister for pk=" # pk);
            await _createServiceCanister(pk, ?[owner, Principal.fromActor(this)]);
        } else {
            throw Error.reject("not authorized");
        };
    };

    // Partition Service canisters by the group passed in
    func createServiceCanister(pk : Text) : async ?Text {
        let canisterIds = getCanisterIdsIfExists(pk);
        if (canisterIds == []) {
            ?(await _createServiceCanister(pk, ?[owner, Principal.fromActor(this)]));
            // the partition already exists, so don't create a new canister
        } else {
            Debug.print(pk # " already exists");
            null;
        };
    };

    // Spins up a new Service canister with the provided pk and controllers
    func _createServiceCanister(pk : Text, controllers : ?[Principal]) : async Text {
        Debug.print("creating new service canister with pk=" # pk);
        // Pre-load 300 billion cycles for the creation of a new Service canister
        // Note that canister creation costs 100 billion cycles, meaning there are 200 billion
        // left over for the new canister when it is created
        Cycles.add(300_000_000_000);
        let newServiceCanister = await DB.DB({
            partitionKey = pk;
            scalingOptions = {
                autoScalingHook = autoScaleServiceCanister;
                sizeLimit = #heapSize(950_000_000); // Scale out at 950MB
                // for auto-scaling testing
                //sizeLimit = #count(3); // Scale out at 3 entities inserted
            };
            owners = controllers;
        });
        let newServiceCanisterPrincipal = Principal.fromActor(newServiceCanister);
        await CA.updateCanisterSettings({
            canisterId = newServiceCanisterPrincipal;
            settings = {
                controllers = controllers;
                compute_allocation = ?0;
                memory_allocation = ?0;
                freezing_threshold = ?2592000;
            };
        });

        let newServiceCanisterId = Principal.toText(newServiceCanisterPrincipal);
        // After creating the new Service canister, add it to the pkToCanisterMap
        pkToCanisterMap := CanisterMap.add(pkToCanisterMap, pk, newServiceCanisterId);

        Debug.print("new service canisterId=" # newServiceCanisterId);
        newServiceCanisterId;
    };
};
