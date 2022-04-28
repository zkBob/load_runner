use load_runner::{
    generator::{Deposit, Generator},
    sender::{send_tx, JobResult},
    telemetry::*,
    utils::TestError,
};
use tokio::sync::mpsc;

use std::{
    env, fs,
    io::Write,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration},
};

use clap::Parser;

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
    send: bool,
}

// #[tokio::main]
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

    let (channel_sender, mut rx) = mpsc::channel::<JobResult>(1);

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

    if args.send {
        let txs = fs::read_dir("./txs").unwrap();

        let count = args.count.into();
        for (index, entry) in txs.enumerate() {
            if index > count {
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
    } else {
        match args.tx_type.as_str() {
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
        }
    }
}
