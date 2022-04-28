use load_runner::{
    generator::{Deposit, Generator},
    sender::send_tx,
    telemetry::*,
    utils::TestError,
};

use std::{
    env, fs,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
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
    println!("{:?}", args);

    let sk = env::var("SK").unwrap();
    let generator = Generator::new(sk.as_str());

    let threads: usize = args.threads.into();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_name_fn(|| {
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
            format!("senders-{}", id)
        })
        .worker_threads(threads)
        .enable_all()
        .on_thread_start(|| {
            println!("{:?} init", thread::current().name().unwrap());
        })
        .on_thread_stop(|| {
            println!("{:?} kill", thread::current().name().unwrap());
        })
        .build()
        .unwrap();

    if !args.send {
        //generate lots of tx and save them
        match args.tx_type.as_str() {
            "deposit" => {
                for i in 1..args.count {
                    rt.spawn(async move {
                        generator.generate_deposit().await.unwrap();
                    });
                    tracing::info!("processed {} tx", i);
                }

                Ok(())
            }
            _ => Err(load_runner::utils::TestError::GeneratorError(String::from(
                "unknown transaction type",
            ))),
        }
    } else {
        let txs = fs::read_dir("./txs").unwrap();

        for (index, entry) in txs.enumerate() {
            if index >= args.count.into() {
                break;
            }

            if index % threads == 0 {
                // println!("step {}, threads spawned, scheduled next execution\n", index / threads);
                thread::sleep(Duration::from_millis(1000));
            }

            let tx = entry.unwrap();
            let content = fs::read(tx.path().as_os_str()).unwrap();
            let d: Deposit = serde_json::from_slice::<Deposit>(&content).unwrap();

            rt.spawn(async {
                // sender::emulate_send(d).await.unwrap();

                send_tx(d).await.map_err(|v| TestError::NetworkError(v))
            });
        }
        thread::sleep(Duration::from_millis(1000));
        //sends prebaked transactions
        // let deposit: Deposit = match env::var("DEPOSIT_TX") {
        //     Ok(path) => {
        //         tracing::info!("found path to deposit tx:{}", path);
        //         match fs::read(path) {
        //             Ok(serialized_deposit) => serde_json::from_slice(&serialized_deposit).unwrap(),
        //             _ => {
        //                 tracing::info!("reading failed, generating new tx");
        //                 generate_deposit().await?
        //             }
        //         }
        //     }
        //     _ => {
        //         tracing::info!("generating new tx");
        //         generate_deposit().await?
        //     }
        // };

        // send_tx(deposit)
        //     .await
        //     .map_err(|v| TestError::NetworkError(v))
        Ok(())
    }
}
