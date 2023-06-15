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

shared ({ caller = owner }) actor class IndexRegistryCanister() = this {
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
