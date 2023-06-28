#!/bin/bash

cd $(dirname $0)/..

dfx canister stop --all
dfx canister delete --all
dfx deploy

dfx canister call proxy set_registry $(dfx canister id registry)
dfx canister call proxy register $(dfx canister id call_source)
dfx canister call proxy register $(dfx canister id call_dest)
dfx canister call call_source set_proxy $(dfx canister id proxy)
dfx canister call call_source set_dest $(dfx canister id call_dest)
dfx canister call call_dest set_proxy $(dfx canister id proxy)
dfx canister call call_source example_call

