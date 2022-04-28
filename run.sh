export TRANSFER_PARAMS_PATH=../zeropool-relayer/zp-relayer/params/transfer_params.bin
export VK_PATH=../zeropool-relayer/zp-relayer/params/transfer_verification_key.json
export TX_FOLDER=./txs/
export RUST_LOG=INFO
export SK=6cbed15c793ce57650b9877cf6fa156fbef513c4e6134f022a85b1ffdd59b2a1
export RELAYER_URL=http://localhost:8000

rm result.log
rm ./${TX_FOLDER}/*

touch result.log
BLUE='\033[1;34m' 

echo "${BLUE}GENERATING TRANSACTIONS"
RUST_LOG=INFO cargo run --release --  --threads 2  --count  4 | bunyan
echo "${BLUE}SENDING TRANSACTIONS TO RELAYER"
RUST_LOG=INFO cargo run --release --  --threads 2  --send --count  4 | bunyan