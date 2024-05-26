# Count Xeno the 4th

![](https://upload.wikimedia.org/wikipedia/commons/thumb/a/af/Carl_Gustaf_Mannerheim.jpg/1920px-Carl_Gustaf_Mannerheim.jpg)

Proof of concept for ethberlin. Verifies an APK proof from a foreign curve (Ethereum EVM only has BN254 precompile) in solidity.

## Motivation

For BLS signatures, the EVM currently only has a precompile for BN-254 integrated. However, many other curves are of interest, including for security and performance reasons. This project solves verifying an aggregate BLS signature on Ethereum, where Ethereum doesn’t natively support the curved used via a precompile, such as BLS12 381 and 377. The signature scheme is accountable, and the applications are lit: having an accountable signature scheme permits use cases such as transparent DAOs (with efficient signature aggregation) and trustless bridges (since misbehavers can be slashed).

## Development

### Install dependencies

We require `rust`, [`anvil`](https://book.getfoundry.sh/getting-started/installation) and `solc-select` to be installed in order to run the test.

In order to update the json file with ABI and bytecode in case of solidity file changes, you would need `solc` version 0.7.6:

``` bash
solc-select install 0.7.6
solc-select use 0.7.6
solc apk_verifier.sol --combined-json=abi,bin > apk_verifier.json
# update the json file bytecode and abi sections
solc src/apk_verifier.sol --combined-json=abi,bin --optimize | jq '.contracts."src/apk_verifier.sol:ApkVerifier"' | sed 's/\\"/"/g' > src/verifier_artifact.json
# NOTE: you might need to edit the json file to remove the extra quotes
# around abi array
```

### Run tests

To run it, first navigate the rust crate folder and then:

```bash
cargo test


To run, call

``` sh
cd count-xeno-the-4th
cargo test solidity_verifier
>>>>>>> 1d107a8c3c91075c20fedd9b4dc6f5ce1e173dee
```
