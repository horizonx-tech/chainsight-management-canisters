# !bin/bash
cd $(dirname $0)/..

dfx canister create build vault
dfx build vault
dfx deploy initializer
dfx canister call initializer deploy_vault_of
