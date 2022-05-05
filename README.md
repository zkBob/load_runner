## Running the runner
1. Clone relayer, launch setup script to copy circuit params to a local folder
2. Set environment variables ( or pass in the run command):

```
export ZEROPOOL_RELAYER_PATH=PATH/TO/RELAYER
export DEPOSIT_TX=./txs/deposit.json
export VK_PATH=${ZEROPOOL_RELAYER_PATH}/zp-relayer/params/transfer_verification_key.json
export TRANSFER_PARAMS_PATH=${ZEROPOOL_RELAYER_PATH}/zp-relayer/params/transfer_params.bin
export TX_FOLDER=${PATH_TO_SAVE_TX}
export RUST_LOG=info
export SK=${CLIENT_SECRET_KEY}
export RELAYER_URL=http://localhost:8000
```

3. Run
```
RUST_LOG="info" cargo run --release -- --help
```
4. Optionnaly install bunyan

```
cargo install bunyan
```

and run 

```
RUST_LOG="info" cargo run --release | bunyan
```


## Running the visualiztion suite

```
cd docker
docker-compose up -d
```

navigate to http://localhost:9090

## metrics

1. [90-th percentile latency](http://localhost:9090/graph?g0.expr=histogram_quantile(0.9%2C%20rate(task_processing_duration_bucket%5B10m%5D))&g0.tab=0&g0.stacked=1&g0.show_exemplars=1&g0.range_input=5m&g0.step_input=1) 
2. TBD: 90-th percentile vs load