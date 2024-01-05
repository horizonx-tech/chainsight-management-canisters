.PHONY: all

include ../Makefile

all:
	dfx canister create _management_canister_registry --specified-id $(ID_REGISTRY) --network http://localhost:$(port)
	dfx build _management_canister_registry --network http://localhost:$(port)
	dfx canister create _management_canister_initializer --specified-id $(ID_INITIALIZER) --network http://localhost:$(port)
	dfx build _management_canister_initializer --network http://localhost:$(port)
	make install
