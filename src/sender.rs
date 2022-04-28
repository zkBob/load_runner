use std::{
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::StatusCode;

use crate::{generator::Deposit, utils::TestError};

use tokio::time::{sleep, Duration};

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

pub async fn send_tx(deposit: Deposit) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();

    let body = serde_json::to_string(&deposit).unwrap();

    tracing::trace!("tx body:\n{}", body);

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
            tracing::info!("tx sent")
        }
        _ => {
            let response = result.text().await.unwrap();
            tracing::error!("something wrong happened {}", response);
        }
    }
    Ok(())
}