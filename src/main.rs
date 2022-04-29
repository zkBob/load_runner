use load_runner::{
    generator::{Deposit, Generator},
    sender::{send_tx, JobResult, JobStatus},
    telemetry::*,
    utils::TestError,
};
use tokio::{
    runtime::Runtime,
    sync::mpsc,
};

use std::{
    env, fs,
    io::Write,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, UNIX_EPOCH, SystemTime},
};

use clap::Parser;
use lazy_static::lazy_static;
use prometheus::{labels, register_counter, register_histogram, Counter, Histogram};

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
}

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
        "example_push_request_duration_seconds",
        "The push request latencies in seconds."
    )
    .unwrap();
}

fn send(threads: usize, rt: Runtime, limit: usize) -> Result<(), TestError> {
    let txs = fs::read_dir("./txs").unwrap();

    let (channel_sender, mut rx) = mpsc::channel::<JobResult>(1);
    // let count = args.count.into();
    for (index, entry) in txs.enumerate() {
        if index > limit {
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
        rt.spawn(async {
            send_tx(file_name, d, mpsc_sender).await;
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

async fn view_results() -> Result<(), TestError> {
    use std::fs::File;
    use std::io::{prelude::*, BufReader};

    let batch_size = 100;

    let file = File::open("result.log")?;
    let reader = BufReader::new(file);
    let mut batch: Vec<f64> = vec![];
    for (index, line) in reader.lines().enumerate() {
        let job_result: JobResult = serde_json::from_slice(line.unwrap().as_bytes()).unwrap();
        println!("read job info {:?}", job_result);

        let job_status: JobStatus =
            reqwest::get(format!("http://localhost:8000/job/{}", job_result.job_id))
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
        
        tracing::info!("job {}, elapsed {}", job_result.job_id, job_status.elapsed);

        batch.push(job_status.elapsed.into());

        if index > 0 && index % batch_size == 0 {
            publish(&batch);
            batch.clear();
        }
    }

    if !batch.is_empty() {
        tracing::info!("publishing batch {}", batch.len());
        publish(&batch);
    }
    
    Ok(())
}

fn publish(values: &Vec<f64>) {
    let address = "127.0.0.1:9091".to_owned();

    for value in values {

        let name = format!("{:?}",SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());
        PUSH_REQ_HISTOGRAM.observe(*value);
        let metric_families = prometheus::gather();
        prometheus::push_metrics(
            name.as_str() ,
            labels! {"instance".to_owned() => "HAL-9000".to_owned(),},
            &address,
            metric_families,
            Some(prometheus::BasicAuthentication {
                username: "user".to_owned(),
                password: "pass".to_owned(),
            }),
        )
        .unwrap();

        tracing::info!("published {} ", name );
    }
   

    
    // for _ in 0..5 {
    //     thread::sleep(time::Duration::from_secs(2));
    //     PUSH_COUNTER.inc();
    //     let metric_families = prometheus::gather();
    //     let _timer = PUSH_REQ_HISTOGRAM.start_timer(); // drop as observe

    // }
}
fn main() -> Result<(), TestError> {
    init_subscriber(get_subscriber(
        "load_runner".into(),
        "trace".into(),
        std::io::stdout,
    ));

    let args = Args::parse();
    tracing::info!("{:?}", args);

    let sk = env::var("SK").unwrap();
    let generator = Generator::new(sk.as_str());

    let threads: usize = args.threads.into();

    let rt = init_runtime(threads);

    match args.mode.as_str() {
        "generate" => match args.tx_type.as_str() {
            "deposit" => {
                for _ in 1..args.count {
                    rt.spawn(async move {
                        generator.generate_deposit().await.unwrap();
                    });
                }
                Ok(())
            }
            _ => Err(load_runner::utils::TestError::GeneratorError(String::from(
                "unknown transaction type",
            ))),
        },
        "send" => send(threads, rt, args.count.into()),

        "publish" => {
            rt.block_on(async { view_results().await }).unwrap();

            // thread::sleep(Duration::from_millis(20000));
            Ok(())
    },

        _ => Err(TestError::ConfigError(String::from("unknown mode"))),
    }
}
