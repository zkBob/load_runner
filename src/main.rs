use futures_util::stream::FuturesUnordered;
use load_runner::{
    generator::{Deposit, Generator},
    sender::{send_tx, JobResult, JobStatus},
    telemetry::*,
    utils::TestError,
};
use tokio::{runtime::Runtime, sync::mpsc};

use std::{
    env, fs,
    io::Write,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
};

use clap::Parser;
use lazy_static::lazy_static;
use prometheus::{labels, register_counter, register_histogram, Counter, Histogram};

use futures::prelude::*;
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "deposit")]
    tx_type: String,
    #[clap(short, long, default_value = "1")]
    count: u16,
    #[clap(long, default_value = "1")]
    threads: u8,
    #[clap(short, long)]
    mode: String,
    #[clap(long, default_value = "0")]
    skip: u8,
}

const DEFAULT_SK: &str = "6cbed15c793ce57650b9877cf6fa156fbef513c4e6134f022a85b1ffdd59b2a1";
const DEFAULT_RELAYER_URL: &str = "http://localhost:8000";

// #[tokio::main]

fn init_runtime(threads: usize) -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
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
        .unwrap()
}

lazy_static! {
    static ref PUSH_COUNTER: Counter = register_counter!(
        "example_push_total",
        "Total number of prometheus client pushed."
    )
    .unwrap();
    static ref PUSH_REQ_HISTOGRAM: Histogram = register_histogram!(
        "tx_latency",
        "The push request latencies in seconds.",
        vec![3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
    )
    .unwrap();
}

fn send(threads: usize, rt: &Runtime, limit: usize, skip: usize) -> Result<(), TestError> {
    let txs_folder = env::var("TXS_FOLDER").unwrap_or("./txs".to_owned());
    let txs = fs::read_dir(txs_folder).unwrap();

    let (channel_sender, mut rx) = mpsc::channel::<JobResult>(1000);
    // let count = args.count.into();
    for (index, entry) in txs.enumerate() {
        if index < skip {
            continue;
        }
        if index == limit + skip {
            break;
        }

        if index % threads == 0 {
            thread::sleep(Duration::from_millis(1000));
        }

        let tx = entry.unwrap();
        let content = fs::read(tx.path().as_os_str()).unwrap();
        let d: Deposit = serde_json::from_slice::<Deposit>(&content).unwrap();
        let file_name = tx.file_name().to_string_lossy().into_owned();
        let mpsc_sender = channel_sender.clone();
        let relayer_url = env::var("RELAYER_URL").unwrap_or(DEFAULT_RELAYER_URL.to_owned());
        rt.spawn(async {
            send_tx(file_name, d, mpsc_sender, relayer_url).await;
        });
    }

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

    thread::sleep(Duration::from_millis(10000));
    Ok(())
}

async fn view_results() -> Result<Vec<f64>, TestError> {
    use std::fs::File;
    use std::io::{prelude::*, BufReader};

    let file = File::open("result.log")?;
    let reader = BufReader::new(file);
    let relayer_url = env::var("RELAYER_URL").unwrap_or(DEFAULT_RELAYER_URL.to_owned());
    let mut results: Vec<f64> = vec![];
    for (_index, line) in reader.lines().enumerate() {
        let job_result: JobResult = serde_json::from_slice(line.unwrap().as_bytes()).unwrap();

        let job_status: JobStatus =
            reqwest::get(format!("{}/job/{}", relayer_url, job_result.job_id))
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

        let elapsed_sec = f64::from(job_status.elapsed) / 1000.0;
        tracing::info!("job {}, elapsed {}", job_result.job_id, elapsed_sec);

        results.push(elapsed_sec);

        // if index > 0 && index % results_size == 0 {
        //     publish(&results);
        //     results.clear();
        // }
    }

    // if !results.is_empty() {
    //     tracing::info!("publishing results {}", results.len());
    //     publish(&results);
    // }

    Ok(results)
}


fn send_to_gw(index: usize) {
    let address = env::var("PROMETHEUS_PUSH_GW").unwrap_or("http://127.0.0.1:9091".to_owned());
    let job_name = String::from("job"); //???
    let metric_families = prometheus::gather();
            prometheus::push_metrics(
                job_name.as_str(),
                labels! {"instance".to_owned() => "HAL-9000".to_owned(),},
                &address,
                metric_families,
                Some(prometheus::BasicAuthentication {
                    username: "user".to_owned(),
                    password: "pass".to_owned(),
                }),
            )
            .unwrap();

            tracing::info!("published {} ", index);
            thread::sleep(Duration::from_millis(100));
}
fn publish(target: &Histogram, values: &Vec<f64>, batch_size: usize) {

    for (index, value) in values.into_iter().enumerate() {
        target.observe(*value);
        if index % batch_size == 0 {
            send_to_gw(index);
        }
    }
    send_to_gw(values.len());
}
fn main() -> Result<(), TestError> {
    init_subscriber(get_subscriber(
        "load_runner".into(),
        "trace".into(),
        std::io::stdout,
    ));

    let args = Args::parse();
    tracing::info!("{:?}", args);

    let threads: usize = args.threads.into();

    let rt = init_runtime(threads);

    match args.mode.as_str() {
        "generate" => match args.tx_type.as_str() {
            "deposit" => {
                rt.block_on(async {
                    let mut completion_stream = (0..args.count.into())
                        .map(|_| async {
                            let thread_name: String = thread::current().name().unwrap().to_owned();

                            tracing::info!("{} started", thread_name);

                            let sk = env::var("SK").unwrap_or(DEFAULT_SK.to_owned());

                            let generator = Generator::new(sk.as_str());

                            generator.generate_deposit().await.unwrap()
                        })
                        .map(|f| rt.spawn(f))
                        .collect::<FuturesUnordered<_>>();

                    while let Some(Ok((file_name, thread_name))) = completion_stream.next().await {
                        tracing::info!("{} saved {}", thread_name, file_name);
                    }

                    // for _ in 0..args.count {
                    //     let sk = env::var("SK").unwrap_or(DEFAULT_SK.to_owned());
                    //     let generator = Generator::new(sk.as_str());
                    //     rt.spawn(async move { generator.generate_deposit().await.unwrap() });

                    // }
                });
                Ok(())
            }
            _ => Err(load_runner::utils::TestError::GeneratorError(String::from(
                "unknown transaction type",
            ))),
        },
        "send" => rt.block_on(async { send(threads, &rt, args.count.into(), args.skip.into()) }),
        "publish" => {
            let batch_size = env::var("BATCH_SIZE").unwrap_or("1".to_string());
            let results = rt.block_on(async { view_results().await }).unwrap();

            publish(&PUSH_REQ_HISTOGRAM,&results, batch_size.parse::<usize>().unwrap());
            // thread::sleep(Duration::from_millis(10000));
            Ok(())
        }

        _ => Err(TestError::ConfigError(String::from("unknown mode"))),
    }
}

#[test]
fn publish_test() {
    init_subscriber(get_subscriber(
        "load_runner".into(),
        "trace".into(),
        std::io::stdout,
    ));

    lazy_static!{
        static ref TEST_HISTOGRAM: Histogram = register_histogram!(
            "test",
            "The push request latencies in seconds.",
            vec![3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
        )
        .unwrap();
    }
    for _ in 1..3 {
        use rand::Rng;

        let ints: [u8; 32] = rand::thread_rng().gen();

        let mut batch_size: usize = 0;

        while batch_size % 10 == 0 {
            batch_size = (rand::thread_rng().gen::<usize>()) % 10;
        }

        tracing::info!("batch_size {:?}", batch_size);

        // let batch_size: usize = rand::thread_rng().gen();

        let values = ints.map(|e| f64::try_from(e % 10).unwrap());

        tracing::info!("{:?}", values);

        let mut v: Vec<f64> = vec![0.0; 32];

        v.copy_from_slice(&values[..]);

        publish(&TEST_HISTOGRAM,&v, batch_size % 10);
    }
}
