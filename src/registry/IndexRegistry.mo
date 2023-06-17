import Cycles "mo:base/ExperimentalCycles";
import Debug "mo:base/Debug";
import Error "mo:base/Error";
import Principal "mo:base/Principal";
import Text "mo:base/Text";

import CA "mo:candb/CanisterActions";
import CanisterMap "mo:candb/CanisterMap";
import Utils "mo:candb/Utils";
import Buffer "mo:stable-buffer/StableBuffer";
import IndexService "./IndexService";
import Canister "Canister";
import CanDB "mo:candb/CanDB";
import Entity "mo:candb/Entity";
import CanisterRepository "CanisterRepository";
import Array "mo:base/Array";
import Log "Log";
import LogRepository "LogRepository";
import Time "mo:base/Time";

shared ({ caller = owner }) actor class IndexRegistryCanister() = this {
    type DB = actor {
        put : shared (opts : CanDB.PutOptions) -> async ();
        get : shared (opts : CanDB.GetOptions) -> async (?Entity.Entity);
        scan : shared (opts : CanDB.ScanOptions) -> async (CanDB.ScanResult);
    };
    type CanisterRepositoryIFace = {
        put : (canister : Canister.Canister) -> async ();
        get : (principal : Principal) -> async (?Canister.Canister);
    };
    type LogRepositoryIFace = {
        putCallLog : Log.CallLog -> async ();
        listCallLogsBetween : (canister : Canister.Canister, from : Time.Time, to : ?Time.Time) -> async ([Log.CallLog]);
    };

    func listLogRepositories() : [LogRepositoryIFace] {
        let canisterIds = getCanisterIdsIfExists("Logs");
        Array.map<Text, LogRepositoryIFace>(
            canisterIds,
            func(canisterId) {
                let act = actor (canisterId) : DB;
                LogRepository.Repository(act);
            },
        );
    };

    func listCanisterRepositories() : [CanisterRepositoryIFace] {
        let canisterIds = getCanisterIdsIfExists("Canisters");
        Array.map<Text, CanisterRepositoryIFace>(
            canisterIds,
            func(canisterId) {
                let act = actor (canisterId) : DB;
                CanisterRepository.Repository(act);
            },
        );
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

    public shared func put() : async () {
        await listCanisterRepositories()[0].put(Canister.newCanister(Principal.fromActor(this)));
    };

    public shared func registerCanister(principal : Principal) : async () {
        await listCanisterRepositories()[0].put(Canister.newCanister(principal));
    };

    public shared func get() : async ?Canister.Canister {
        await listCanisterRepositories()[0].get(Principal.fromActor(this));
    };

    public shared func debugPutLog() : async () {
        let can = Canister.newCanister(Principal.fromActor(this));
        await listLogRepositories()[0].putCallLog(Log.newCallLog(can, can));
    };

    public shared func putLog(caller : Principal, callTo : Principal) : async () {
        await listLogRepositories()[0].putCallLog(Log.newCallLog(Canister.newCanister(caller), Canister.newCanister(callTo)));
    };

    public shared func listLogsOf(principal : Principal, from : Time.Time, to : Time.Time) : async ([Log.CallLog]) {
        await listLogRepositories()[0].listCallLogsBetween(Canister.newCanister(principal), from, ?to);
    };

    public shared func exists(principal : Principal) : async Bool {
        for (repo in listCanisterRepositories().vals()) {
            if ((await repo.get(principal)) != null) {
                return true;
            };
        };
        return false;
    };

    public shared (msg) func init() : async [?Text] {
        assert (owner == msg.caller);
        ([
            await createServiceCanister("Canisters"),
            await createServiceCanister("Logs"),
        ]);
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
        let newServiceCanister = await IndexService.IndexService({
            partitionKey = pk;
            scalingOptions = {
                autoScalingHook = autoScaleServiceCanister;
                sizeLimit = #heapSize(475_000_000); // Scale out at 475MB
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
