1. Clone relayer, launch setup script to copy circuit params to a local folder
2. Set environment variables ( or pass in the run command):

```
export ZEROPOOL_RELAYER_PATH=PATH/TO/RELAYER
export DEPOSIT_TX=./txs/deposit.json
export VK_PATH=${ZEROPOOL_RELAYER_PATH}/zp-relayer/params/transfer_verification_key.json
export TRANSFER_PARAMS_PATH=${ZEROPOOL_RELAYER_PATH}/zp-relayer/params/transfer_params.bin
```

3. Run
```
RUST_LOG="info" cargo run --release
```
4. Optionnaly install bunyan

```
cargo install bunyan
```

and run 

```
RUST_LOG="info" cargo run --release | bunyan
```