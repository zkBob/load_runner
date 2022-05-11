use std::{
    env, fs,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{generator::Deposit, utils::TestError};

use tokio::{
    runtime::Runtime,
    sync::mpsc::{self, Sender},
    time::Duration,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: u32,
    file_name: String,
    created: SystemTime,
}

#[derive(Debug, Deserialize)]
pub struct JobStatus {
    #[serde(rename(deserialize = "state"))]
    _state: String,
    #[serde(rename(deserialize = "txHash"))]
    _tx_hash: String,
    #[serde(rename(deserialize = "created"))]
    _created: u64,
    pub elapsed: u32,
}

#[derive(Debug, Deserialize)]
struct RelayerReponse {
    #[serde(rename(deserialize = "jobId"))]
    job_id: String,
}

const DEFAULT_RELAYER_URL: &str = "http://localhost:8000";

pub fn send(tps: usize, rt: &Runtime, limit: usize, skip: usize) -> Result<(), TestError> {
    use std::io::Write;

    let txs_folder = env::var("TXS_FOLDER").unwrap_or("./txs".to_owned());
    let mut txs = fs::read_dir(txs_folder).unwrap();

    let (channel_sender, mut rx) = mpsc::channel::<JobResult>(1000);
    let mut count = 0;

    let start = SystemTime::now();

    let step: f64 = 1.0 / (tps as f64);
    let mut total: f64 = 0.0;
    let mut skipped: usize = skip;

    let _rx_handle = rt.spawn(async move {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open("result.log")
            .unwrap();
        // Start receiving messages
        while let Some(job_result) = rx.recv().await {
            let content = serde_json::to_string(&job_result).unwrap();
            tracing::info!("received job result {}", content);
            if let Err(e) = writeln!(file, "{}", content) {
                eprintln!("Couldn't write to file: {}", e);
            }
        }
    });
    
    loop {
        let elapsed = start.elapsed().unwrap();

        if elapsed.as_secs() as f64 > total {
            if count == limit {
                break;
            }
            while skipped > 0 {
                _ = txs.next();
                skipped -= 1;
            }

            if let Some(Ok(tx)) = txs.next() {
                count += 1;
                let mpsc_sender = channel_sender.clone();
                rt.spawn(async move {
                    let content = fs::read(tx.path().as_os_str()).unwrap();
                    let d: Deposit = serde_json::from_slice::<Deposit>(&content).unwrap();
                    let file_name = tx.file_name().to_string_lossy().into_owned();
                    let relayer_url =
                        env::var("RELAYER_URL").unwrap_or(DEFAULT_RELAYER_URL.to_owned());
                    send_tx(file_name, d, mpsc_sender, relayer_url).await;
                });
            } else {
                break;
            }

            total += step;
        }
    }
    
    // thread::sleep(Duration::from_millis(10000));
    Ok(())
}

pub async fn send_tx(
    file_name: String,
    deposit: Deposit,
    mpsc_sender: Sender<JobResult>,
    relayer_url: String,
) -> () {
    let client = reqwest::Client::new();

    let body = serde_json::to_string(&deposit).unwrap();

    tracing::debug!(
        "{} started at {:?}",
        thread::current().name().unwrap(),
        SystemTime::now().duration_since(UNIX_EPOCH)
    );
    tracing::trace!("tx body:\n{}", body);

    let result = client
        .post(format!("{}/transaction", relayer_url))
        .body(body)
        .header("Content-type", "application/json")
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .unwrap();
    // .map_err(|e| TestError::NetworkError(e))?;

    match result.status() {
        StatusCode::OK => {
            let response = result.json::<RelayerReponse>().await.unwrap();
            tracing::info!("tx response {:#?}", response);
            let job_id: u32 = response.job_id.parse::<u32>().unwrap();
            mpsc_sender
                .send(JobResult {
                    job_id,
                    file_name,
                    created: SystemTime::now(),
                })
                .await
                .unwrap();
        }
        _ => {
            let response = result.text().await.unwrap();
            tracing::error!("something wrong happened {}", response);
        }
    }
}

mod test {

    #[test]
    fn test_send() {
        use tokio::time::sleep;

        use crate::utils::TestError;

        use std::{
            sync::atomic::{AtomicUsize, Ordering},
            thread,
            time::{Duration, SystemTime},
        };

        async fn emulate_send(index: i32) -> Result<(), TestError> {
            let handle = thread::current();
            tracing::debug!("step {} {:?} start", index, handle.name().unwrap());
            sleep(Duration::from_millis(1000)).await;
            Ok(())
        }

        let threads = 1;

        crate::telemetry::init_subscriber(crate::telemetry::get_subscriber(
            "load_runner".into(),
            "trace".into(),
            std::io::stdout,
        ));

        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_name_fn(|| {
                static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                format!("senders-{}", id)
            })
            .worker_threads(threads)
            .enable_all()
            .on_thread_start(|| {
                tracing::debug!("{:?} init", thread::current().name().unwrap());
            })
            .on_thread_stop(|| {
                tracing::debug!("{:?} kill", thread::current().name().unwrap());
            })
            .build()
            .unwrap();

        rt.block_on(async {
            let start = SystemTime::now();

            let mut step: i32 = 0;
            loop {
                let elapsed = start.elapsed().unwrap();

                if elapsed.as_secs() > step.try_into().unwrap() {
                    rt.spawn(async move {
                        emulate_send(step).await.unwrap();
                    });
                    step += 1;
                }

                if step == 10 {
                    break;
                }
            }
        })
    }
}
