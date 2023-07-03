import TimeStampedSK "../src/registry/db/TimeStampedSK";
import ActorSpec "./utils/ActorSpec";
import Debug "mo:base/Debug";
import Principal "mo:base/Principal";
type Group = ActorSpec.Group;

let assertTrue = ActorSpec.assertTrue;
let describe = ActorSpec.describe;
let it = ActorSpec.it;
let pending = ActorSpec.pending;
let run = ActorSpec.run;
let canisterId = "be2us-64aaa-aaaaa-qaabq-cai";
let principal = Principal.fromText(canisterId);
let success = run([
    describe(
        "TimeStampedSK",
        [
            it(
                "callLogSK",
                do {
                    assertTrue(TimeStampedSK.callLogSK(principal, 123) == "CallLog:" #canisterId # ":123");
                },
            ),
            it(
                "calledLogSK",
                do {
                    assertTrue(TimeStampedSK.calledLogSK(principal, 456) == "CalledLog:" #canisterId # ":456");
                },
            ),
            //it(
            //    "fromSKText",
            //    do {
            //        assertTrue(TimeStampedSK.fromSKText("CallLog:" #canisterId # ":123") == TimeStampedSK.newTimeStampedSK("CallLog", principal, 123));
            //    },

            //),
        ],
    )

]);

if (success == false) {
    Debug.trap("Tests failed");
};
