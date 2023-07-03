import Principal "mo:base/Principal";
import Time "mo:base/Time";
import Int "mo:base/Int";
import Text "mo:base/Text";
import Iter "mo:base/Iter";
import Nat32 "mo:base/Nat32";
import Char "mo:base/Char";
import Debug "mo:base/Debug";

module TimeStampedSK {
    type TimeStampedSK = {
        prefix : Text;
        id : Text;
        time : Time.Time;
    };

    let delimiter : Text = ":";

    public func callLogSK(canisterId : Principal, time : Time.Time) : Text {
        let current = Time.now();
        toText(newTimeStampedSK("CallLog", Principal.toText(canisterId), time));
    };

    public func calledLogSK(canisterId : Principal, time : Time.Time) : Text {
        let current = Time.now();
        toText(newTimeStampedSK("CalledLog", Principal.toText(canisterId), time));
    };

    func newTimeStampedSK(prefix : Text, id : Text, time : Time.Time) : TimeStampedSK {
        {
            prefix = prefix;
            id = id;
            time = time;
        };
    };

    func toText(ts : TimeStampedSK) : Text {
        Debug.print(ts.id);
        ts.prefix # delimiter # ts.id # delimiter # Int.toText(ts.time);
    };

    public func fromSKText(txt : Text) : TimeStampedSK {
        let parts = Iter.toArray(Text.split(txt, #text delimiter));
        {
            prefix = parts[0];
            id = parts[1];
            time = textToNat(parts[2]);
        };
    };

    func textToNat(txt : Text) : Nat {
        assert (txt.size() > 0);
        let chars = txt.chars();

        var num : Nat = 0;
        for (v in chars) {
            let charToNum = Nat32.toNat(Char.toNat32(v) -48);
            assert (charToNum >= 0 and charToNum <= 9);
            num := num * 10 + charToNum;
        };
        num;
    };
};
