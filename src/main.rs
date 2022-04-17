use libzeropool::{
    fawkes_crypto::{
        backend::bellman_groth16::{engines::Bn256, prover},
        engines::bn256::Fr,
        ff_uint::Num,
    },
    native::boundednum::BoundedNum,
};
use libzeropool_rs::client::{state::State, TxType, UserAccount};
use load_runner::telemetry::*;
use reqwest::StatusCode;
use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use std::{env, time::Duration};
use std::{fs, str::FromStr};
use web3::{api::Accounts, types::SignedData};

#[derive(Debug)]
enum TestError {
    NetworkError(reqwest::Error),
    Web3Error(web3::ethabi::Error),
    SerializationError(serde_json::Error),
}

#[derive(Serialize, Deserialize)]
struct Proof {
    inputs: Vec<Num<Fr>>,
    proof: prover::Proof<Bn256>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Deposit {
    proof: Proof,
    memo: Vec<u8>,
    tx_type: String,
    deposit_signature: String,
}

#[tokio::main]
async fn main() -> Result<(), TestError> {
    init_subscriber(get_subscriber(
        "load_runner".into(),
        "trace".into(),
        std::io::stdout,
    ));

    let deposit: Deposit = match env::var("DEPOSIT_TX") {
        Ok(path) => {
            tracing::info!("found path to deposit tx:{}", path);
            match fs::read(path) {
                Ok(serialized_deposit) => serde_json::from_slice(&serialized_deposit).unwrap(),
                _ => {
                    tracing::info!("reading failed, generating new tx");
                    generate_deposit().await?
                }
            }
        }
        _ => {
            tracing::info!("generating new tx");
            generate_deposit().await?
        }
    };

    tracing::info!("sending tx to relayer");
    send_tx(deposit)
        .await
        .map_err(|v| TestError::NetworkError(v))
}

async fn send_tx(deposit: Deposit) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();

    let body = serde_json::to_string(&deposit).unwrap();

    println!("{}", body);

    // let body = "{\"foo\":\"bar\"}";
    let result = client
        .post("http://localhost:8000/transaction")
        .body(body)
        .header("Content-type", "application/json")
        .timeout(Duration::from_secs(3))
        .send()
        .await?;

    match result.status() {
        StatusCode::OK => {
            tracing::info!("tx processed")
        }
        _ => {
            let response = result.text().await.unwrap();
            tracing::error!("something wrong happened {}", response);
        }
    }

    // println!("body = {:?}", body);

    Ok(())
}

fn pack_signature(signature: SignedData) -> Result<String, TestError> {
    let r = serde_json::to_string(&signature.r).map_err(|e| TestError::SerializationError(e))?;

    let s = serde_json::to_string(&signature.s).map_err(|e| TestError::SerializationError(e))?;

    let mut packed = String::from(&r[1..67]);

    packed.push_str(&s[3..67]);

    if signature.v % 2 != 0 {
        packed.push_str("1");
    }
    tracing::trace!(
        "Signature:\nv:{},\n{:#?}\n{:#?},\n{:#?}",
        signature.v,
        &signature.r,
        &signature.s,
        &packed
    );

    Ok(packed)
}
fn sign_nullifier(nullifier: Num<Fr>) -> Result<String, TestError> {
    use borsh::BorshSerialize;
    let mut buf: [u8; 32] = [0; 32];

    BorshSerialize::serialize(&nullifier, &mut &mut buf[0..32]).unwrap();

    let transport = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(transport);

    // Insert the 32-byte private key in hex format (do NOT prefix with 0x)
    let prvk: secp256k1::SecretKey =
        SecretKey::from_str("01010101010101010001020304050607ffff0000ffff00006363636363636363")
            .unwrap();

    let key_ref = web3::signing::SecretKeyRef::new(&prvk);

    let accounts = web3.accounts();
    let signed = Accounts::sign(&accounts, buf, key_ref);

    pack_signature(signed)
}

async fn generate_deposit() -> Result<Deposit, TestError> {
    use libzeropool::fawkes_crypto::backend::bellman_groth16::verifier::VK;
    use libzeropool::fawkes_crypto::backend::bellman_groth16::{verifier::verify, Parameters};
    use libzeropool::POOL_PARAMS;
    use libzeropool_rs::proof::prove_tx;
    use rand::Rng;

    let state = State::init_test(POOL_PARAMS.clone());
    let acc = UserAccount::new(
        Num::from(rand::thread_rng().gen::<u64>()),
        state,
        POOL_PARAMS.clone(),
    );

    let tx_data = acc
        .create_tx(
            TxType::Deposit(
                BoundedNum::new(Num::ZERO),
                vec![],
                BoundedNum::new(Num::ONE),
            ),
            None,
        )
        .unwrap();

    let params_path = std::env::var("TRANSFER_PARAMS_PATH")
        .unwrap_or(String::from("../params/transfer_params.bin"));

    let vk_path = std::env::var("VK_PATH")
        .unwrap_or(String::from("../params/transfer_verification_key.json"));

    let params_data = std::fs::read(params_path).unwrap();
    let mut params_data_cur = &params_data[..];

    let params = Parameters::<Bn256>::read(&mut params_data_cur, false, false).unwrap();

    let nullifier: Num<libzeropool::fawkes_crypto::engines::bn256::Fr> = tx_data.public.nullifier;
    let (inputs, proof) = prove_tx(&params, &*POOL_PARAMS, tx_data.public, tx_data.secret);

    let vk_str = std::fs::read_to_string(vk_path).unwrap();

    let vk: VK<Bn256> = serde_json::from_str(&vk_str).unwrap();

    let verification_result = verify(&vk, &proof, &inputs);

    assert!(verification_result);

    let deposit_signature = sign_nullifier(nullifier)?;

    let deposit = Deposit {
        proof: Proof { inputs, proof },
        memo: tx_data.memo,
        tx_type: String::from("0000"),
        deposit_signature,
    };

    if env::var("DEPOSIT_TX").is_ok() {
        let tx_folder = env::var("DEPOSIT_TX").unwrap();
        let serialized_deposit = serde_json::to_string(&deposit).unwrap();
        fs::write(tx_folder, serialized_deposit).unwrap();
    }

    Ok(deposit)
}
