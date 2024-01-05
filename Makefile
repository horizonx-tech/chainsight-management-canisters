ID_PROXY=u3zgx-4yaaa-aaaal-achaa-cai
ID_VAULT=ua42s-gaaaa-aaaal-achcq-cai
ID_REGISTRY=uh54g-lyaaa-aaaal-achca-cai
ID_INITIALIZER=7fpuj-hqaaa-aaaal-acg7q-cai

.PHONY: test create-build install local

test:
	$(shell vessel bin)/moc -r $(shell vessel sources) -wasi-system-api test/*Test.mo

create:
	dfx canister create _management_canister_proxy --specified-id $(ID_PROXY) --network http://localhost:$(port)
	dfx canister create _management_canister_vault --specified-id $(ID_VAULT) --network http://localhost:$(port)
	dfx canister create _management_canister_registry --specified-id $(ID_REGISTRY) --network http://localhost:$(port)

build:
	dfx build _management_canister_proxy --network http://localhost:$(port)
	dfx build _management_canister_vault --network http://localhost:$(port)
	dfx build _management_canister_registry --network http://localhost:$(port)
	cp ./.dfx/http___localhost_$(port)/canisters/_management_canister_proxy/_management_canister_proxy.wasm.gz ./artifacts/proxy.wasm.gz
	cp ./.dfx/http___localhost_$(port)/canisters/_management_canister_vault/_management_canister_vault.wasm.gz ./artifacts/vault.wasm.gz
	cp ./.dfx/http___localhost_$(port)/canisters/_management_canister_registry/_management_canister_registry.wasm.gz ./artifacts/registry.wasm.gz
	cp ./.dfx/http___localhost_$(port)/canisters/_management_canister_registry/service.did ./artifacts/registry.did

# NOTE: initializer has other component wasm
create-initializer:
	dfx canister create _management_canister_initializer --specified-id $(ID_INITIALIZER) --network http://localhost:$(port)

build-initializer:
	dfx build _management_canister_initializer --network http://localhost:$(port)
	cp ./.dfx/http___localhost_$(port)/canisters/_management_canister_initializer/_management_canister_initializer.wasm.gz ./artifacts/initializer.wasm.gz

create-build:
	make create build create-initializer build-initializer

install:
	dfx deploy _management_canister_initializer --network http://localhost:$(port)
	dfx deploy _management_canister_registry --network http://localhost:$(port)
	dfx canister call _management_canister_registry init --network http://localhost:$(port)
	dfx canister call _management_canister_initializer set_registry '(principal "$(ID_REGISTRY)")' --network http://localhost:$(port)

local:
	make generate-did create-build install
	dfx canister deposit-cycles 30000000000000 _management_canister_initializer --network http://localhost:$(port)

upgrade:
	make build build-initializer
	dfx canister install _management_canister_initializer --mode=upgrade --network http://localhost:${port}

generate-did:
	cargo test generate_candid
