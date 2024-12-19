let upstream =
      https://github.com/dfinity/vessel-package-set/releases/download/mo-0.12.1-20240808/package-set.dhall
let Package = { name : Text, version : Text, repo : Text, dependencies : List Text }

let packages = [
  { name = "stable-rbtree"
  , repo = "https://github.com/canscale/StableRBTree"
  , version = "v0.6.1"
  , dependencies = [ "base" ]
  },
  { name = "stable-buffer"
  , repo = "https://github.com/canscale/StableBuffer"
  , version = "v0.2.0"
  , dependencies = [ "base" ]
  },
  { name = "btree"
  , repo = "https://github.com/canscale/StableHeapBTreeMap"
  , version = "v0.3.2"
  , dependencies = [ "base" ]
  },
  { name = "candb"
  , repo = "git@github.com:canscale/CanDB.git"
  , version = "beta"
  , dependencies = [ "base" ]
  },
  { name = "candy"
  , repo = "git@github.com:icdevs/candy_library.git"
  , version = "0.2.0"
  , dependencies = [ "base" ]
  },
  { name = "stablebuffer"
  , repo = "https://github.com/skilesare/StableBuffer"
  , version = "v0.2.0"
  , dependencies = [ "base"]
  },
  { name = "map7"
  , repo = "https://github.com/ZhenyaUsenko/motoko-hash-map"
  , version = "v7.0.0"
  , dependencies = [ "base"]
  },]
  : List Package

in  upstream # packages 