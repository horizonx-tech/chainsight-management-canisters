# chainsight-management-canisters

An example of recording call logs from a source canister to a destination canister.
see: [test script](./scripts/test.sh).

## Image

![proxy](./proxy.png)

## For Developer

Two options are available for handling in the local environment.

Option 1: Perform from build to deploy (with Vessel)

- Details can be found in the following files
  - configuration: `dfx.json`
  - commands: `Makefile`

```bash
# with the dfx local node running
make local
```

Option 2: Deploy only

- Details can be found in the following files
  - configuration: `artifacts/dfx.json`
  - commands: `artifacts/Makefile`
- NOTE: This option does not take into account modifications to your management-canister

```bash
# with the dfx local node running
cd artifacts
make all
```
