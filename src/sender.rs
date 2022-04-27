use std::time::Duration;

use reqwest::StatusCode;

use crate::generator::Deposit;



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