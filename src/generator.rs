use libzeropool::{
    fawkes_crypto::{
        backend::bellman_groth16::{engines::Bn256, prover},
        engines::bn256::Fr,
        ff_uint::Num,
    },
    native::boundednum::BoundedNum,
};
use serde::{Deserialize, Serialize};

use libzeropool::fawkes_crypto::backend::bellman_groth16::verifier::VK;
use libzeropool::fawkes_crypto::backend::bellman_groth16::{verifier::verify, Parameters};
use libzeropool::POOL_PARAMS;
use libzeropool_rs::proof::prove_tx;

use libzeropool_rs::client::{state::State, TxType, UserAccount};
use rand::Rng;

use secp256k1::SecretKey;

use std::env;
use std::{fs, str::FromStr};
use web3::{api::Accounts, types::SignedData};

use crate::utils::TestError;

#[derive(Serialize, Deserialize)]
struct Proof {
    inputs: Vec<Num<Fr>>,
    proof: prover::Proof<Bn256>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deposit {
    proof: Proof,
    pub memo: String,
    tx_type: String,
    deposit_signature: String,
}


#[derive(Clone, Copy)]
pub struct Generator {
    sk: SecretKey,
}

fn serialize(num: Num<Fr>) -> Result<[u8; 32], TestError> {
    use borsh::BorshSerialize;

    let mut buf: [u8; 32] = [0; 32];

    BorshSerialize::serialize(&num, &mut &mut buf[0..32]).unwrap();

    buf.reverse();

    Ok(buf)
}

fn pack_signature(signature: &SignedData) -> Result<String, TestError> {
    let mut packed = String::from("0x");
    packed.push_str(&hex::encode(signature.r.as_bytes()));

    let mut s_bytes: [u8; 32] = [0; 32];
    s_bytes.copy_from_slice(signature.s.as_bytes());
    if signature.v % 2 == 0 {
        let first_byte = s_bytes.first_mut().unwrap();
        *first_byte ^= 0b1000_0000;
    }

    packed.push_str(&hex::encode(s_bytes));
    tracing::trace!(
        "Signature:\nv:{},\n{:#?}\n{:#?},\n{:#?}",
        signature.v,
        &signature.r,
        &signature.s,
        &packed
    );

    Ok(packed)
}

impl Generator {
    pub fn new(key: &str) -> Self {
        let sk: secp256k1::SecretKey = SecretKey::from_str(key).unwrap();
        Generator { sk }
    }

    fn sign(&self, buf: [u8; 32]) -> SignedData {
        let transport = web3::transports::Http::new("http://localhost:8545").unwrap();
        let web3 = web3::Web3::new(transport);

        // Insert the 32-byte private key in hex format (do NOT prefix with 0x)
        // let sk: secp256k1::SecretKey = SecretKey::from_str(CLIENT_PK).unwrap();

        let key_ref = web3::signing::SecretKeyRef::new(&self.sk);

        let accounts = web3.accounts();
        let signed = Accounts::sign(&accounts, buf, key_ref);
        signed
    }

pub async fn generate_deposit(&self) -> Result<(), TestError> {
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

        let nullifier: Num<libzeropool::fawkes_crypto::engines::bn256::Fr> =
            tx_data.public.nullifier;
        let (inputs, proof) = prove_tx(&params, &*POOL_PARAMS, tx_data.public, tx_data.secret);

        let vk_str = std::fs::read_to_string(vk_path).unwrap();

        let vk: VK<Bn256> = serde_json::from_str(&vk_str).unwrap();

        let verification_result = verify(&vk, &proof, &inputs);

        assert!(verification_result);

        let nullifier_bytes = serialize(nullifier)?;

        let deposit_signature = self.sign(nullifier_bytes);

        let packed_sig = pack_signature(&deposit_signature)?;

        let deposit = Deposit {
            proof: Proof { inputs, proof },
            memo: hex::encode(tx_data.memo),
            tx_type: String::from("0000"),
            deposit_signature: packed_sig,
        };

        match env::var("TX_FOLDER") {
            Ok(mut tx_folder) => {
                if tx_folder.ends_with("/") {
                    tx_folder.pop();
                }
                let serialized_deposit = serde_json::to_string(&deposit).unwrap();//TODO: error conversion impl
                let path = format!("{}/{}.json", tx_folder, &hex::encode(nullifier_bytes));
                fs::write(path, serialized_deposit).map_err(|e| TestError::SavingError(e))
            }
            Err(_) => Err(TestError::ConfigError(String::from("TX_FOLDER not set"))),
        }
    }
}

#[test]
fn nullifier_sign_test() {
    let mut unhexed =
        hex::decode("22873c1e5b345e0f0b9968cad056e5175603767e12d79b6f58ac15470177e7d4").unwrap();

    unhexed.reverse();

    let nullifier: Num<Fr> = borsh::BorshDeserialize::deserialize(&mut &unhexed[..]).unwrap();

    let nullifier_bytes = serialize(nullifier).unwrap();

    let deposit_signature =
        Generator::new("6cbed15c793ce57650b9877cf6fa156fbef513c4e6134f022a85b1ffdd59b2a1")
            .sign(nullifier_bytes);

    let packed_sig = pack_signature(&deposit_signature).unwrap();

    assert_eq!(packed_sig.as_str(), "0xf70f2aa887c1f146e14a2fe5581805c6f93f99396e4f738740cf45a7af21d54c62ff74ee8b0712c0fe9bd0f94e71b9e2ccde83e7f1dff6c6f91c12a556eb014d");
}

#[test]
fn verifiy_sig() {
    const CLIENT_PUB_KEY: &str = "ffcf8fdee72ac11b5c542428b35eef5769c409f0";

    let mut msg: [u8; 32] = [0; 32];
    let msg_vec =
        hex::decode("22873c1e5b345e0f0b9968cad056e5175603767e12d79b6f58ac15470177e7d4").unwrap();

    msg.copy_from_slice(&msg_vec[0..32]);

    let signed_data =
        Generator::new("6cbed15c793ce57650b9877cf6fa156fbef513c4e6134f022a85b1ffdd59b2a1")
            .sign(msg);

    let r = signed_data.r;

    let s = signed_data.s;

    let v: u64 = signed_data.v.into();

    let transport = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(transport);
    let accounts = web3.accounts();

    println!("r:{:#?}\n,s:{:#?},\nv:{:#?}", r, s, v);

    let packed_sig = pack_signature(&signed_data).unwrap();
    println!("packed : {:#?}", packed_sig);
    let result = accounts.recover(web3::types::Recovery {
        message: web3::types::RecoveryMessage::Data(msg.to_vec()),
        v,
        r,
        s,
    });

    assert_eq!(packed_sig,"0xf70f2aa887c1f146e14a2fe5581805c6f93f99396e4f738740cf45a7af21d54c62ff74ee8b0712c0fe9bd0f94e71b9e2ccde83e7f1dff6c6f91c12a556eb014d");

    let mut client_address: [u8; 20] = [0; 20];

    client_address.copy_from_slice(&hex::decode(CLIENT_PUB_KEY).unwrap());

    assert!(result.is_ok());

    assert_eq!(result.unwrap(), web3::types::H160::from(client_address));
}
