use std::str::FromStr;

use libzeropool::{fawkes_crypto::ff_uint::Num, native::boundednum::BoundedNum};
use libzeropool_rs::client::{state::State, UserAccount, TxType};
use secp256k1::SecretKey;
use web3::{types::{Address, TransactionParameters, U256}, api::Accounts, ethabi::Error};


#[tokio::main]
async fn main() -> Result<(), Error> {
    // Sign up at infura > choose the desired network (eg Rinkeby) > copy the endpoint url into the below
   make_deposit().await;

    Ok(())
}


async fn make_deposit() {
    use libzeropool_rs::proof::prove_tx;
    use borsh::BorshSerialize;
    use libzeropool::fawkes_crypto::backend::bellman_groth16::verifier::VK;
    use libzeropool::fawkes_crypto::backend::bellman_groth16::{
        engines::Bn256, verifier::verify, Parameters,
    };
    use libzeropool::POOL_PARAMS;
    use rand::Rng;

    let state = State::init_test(POOL_PARAMS.clone());
    let random_u64: u64 = rand::thread_rng().gen();
    let acc = UserAccount::new(Num::from(random_u64), state, POOL_PARAMS.clone());

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

    let nullifier: Num<libzeropool::fawkes_crypto::engines::bn256::Fr> =
        tx_data.public.nullifier;
    let (inputs, snark_proof) =
        prove_tx(&params, &*POOL_PARAMS, tx_data.public, tx_data.secret);

    let vk_str = std::fs::read_to_string(vk_path).unwrap();

    let vk: VK<Bn256> = serde_json::from_str(&vk_str).unwrap();

    let verification_result = verify(&vk, &snark_proof, &inputs);

    assert!(verification_result);

    let mut buf: [u8; 32] = [0; 32];

    BorshSerialize::serialize(&nullifier, &mut &mut buf[0..32]).unwrap();

    let transport = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(transport);

    // Insert the 32-byte private key in hex format (do NOT prefix with 0x)
    let prvk:secp256k1::SecretKey = SecretKey::from_str("01010101010101010001020304050607ffff0000ffff00006363636363636363").unwrap();

    let key_ref = web3::signing::SecretKeyRef::new(&prvk);

    let accounts = web3.accounts();
    let signed =  Accounts::sign(&accounts, buf, key_ref);

}