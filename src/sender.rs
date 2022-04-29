use std::{
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::StatusCode;
use serde::{Serialize, Deserialize};

use crate::{generator::Deposit, utils::TestError};

use tokio::{
    sync::mpsc::Sender,
    time::{sleep, Duration},
};

#[derive(Debug,Serialize,Deserialize)]
pub struct JobResult{
    pub job_id: u32,
    file_name: String,
    created: SystemTime
}

#[derive(Debug,Deserialize)]
pub struct JobStatus {
 state: String,
 #[serde(rename(deserialize="txHash"))]
 tx_hash: String,
 created: u64,
pub elapsed: u32
}

#[derive(Debug,Deserialize)]
struct RelayerReponse {
    #[serde(rename(deserialize = "jobId"))]
    job_id: String
}
pub async fn emulate_send(_: Deposit) -> Result<(), TestError> {
    let now = SystemTime::now();

    let handle = thread::current();

    println!(
        "{:?} start {:?}",
        handle.name().unwrap(),
        now.duration_since(UNIX_EPOCH).unwrap(),
    );

    sleep(Duration::from_millis(500)).await;

    println!(
        "{:?} complete {:?} {:?}",
        handle.name().unwrap(),
        now.duration_since(UNIX_EPOCH).unwrap(),
        now.elapsed().unwrap()
    );

    Ok(())
}

pub async fn send_tx(
    file_name: String,
    deposit: Deposit,
    mpsc_sender: Sender<JobResult>,
) -> () {
    let client = reqwest::Client::new();

    let body = serde_json::to_string(&deposit).unwrap();

    tracing::trace!("tx body:\n{}", body);

    let result = client
        .post("http://localhost:8000/transaction")
        .body(body)
        .header("Content-type", "application/json")
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .unwrap();
        // .map_err(|e| TestError::NetworkError(e))?;

    match result.status() {
        StatusCode::OK => {
        
            let response =result.json::<RelayerReponse>().await.unwrap();
            tracing::debug!("tx response {:#?}", response);
            let job_id: u32 = response.job_id.parse::<u32>().unwrap();
            mpsc_sender
                .send(JobResult{job_id, file_name, created:SystemTime::now() })
                .await
                .unwrap();
            
        }
        _ => {
            let response = result.text().await.unwrap();
            tracing::error!("something wrong happened {}", response);
            
        }
    }
}
