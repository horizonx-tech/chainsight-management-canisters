.PHONY: test create install local

test:
	$(shell vessel bin)/moc -r $(shell vessel sources) -wasi-system-api test/*Test.mo

create:
	dfx canister create _management_canister_initializer --specified-id 7fpuj-hqaaa-aaaal-acg7q-cai --network http://localhost:$(port)
	dfx canister create _management_canister_proxy --specified-id u3zgx-4yaaa-aaaal-achaa-cai --network http://localhost:$(port)
	dfx canister create _management_canister_registry --specified-id uh54g-lyaaa-aaaal-achca-cai --network http://localhost:$(port)
	dfx canister create _management_canister_vault --specified-id ua42s-gaaaa-aaaal-achcq-cai --network http://localhost:$(port)
	dfx build --all --network http://localhost:$(port)

install:	
	dfx deploy _management_canister_initializer --network http://localhost:$(port)
	dfx deploy _management_canister_proxy --network http://localhost:$(port)
	dfx deploy _management_canister_registry --network http://localhost:$(port)
	dfx canister call _management_canister_registry init --network http://localhost:$(port)
	dfx canister call _management_canister_proxy set_registry '(principal "uh54g-lyaaa-aaaal-achca-cai")' --network http://localhost:$(port)
	dfx canister call _management_canister_initializer set_registry '(principal "uh54g-lyaaa-aaaal-achca-cai")' --network http://localhost:$(port)
	dfx canister call _management_canister_registry registerProxy '(principal "u3zgx-4yaaa-aaaal-achaa-cai")' --network http://localhost:$(port)
local:
	make create install
