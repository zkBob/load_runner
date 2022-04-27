use load_runner::{generator::Generator, sender::send_tx, telemetry::*, utils::TestError};
use opentelemetry::sdk::export::metrics::Count;

use std::env;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "deposit")]
    tx_type: String,
    #[clap(short, long, default_value = "1")]
    count: u16,
    #[clap( long, default_value = "1")]
    threads: u8,
    #[clap(short, long)]
    send: bool,
}

#[tokio::main]
async fn main() -> Result<(), TestError> {
    init_subscriber(get_subscriber(
        "load_runner".into(),
        "trace".into(),
        std::io::stdout,
    ));

    let args = Args::parse();
    println!("{:?}", args);

    let sk = env::var("SK").unwrap();
    let generator = Generator::new(sk.as_str());

    if !args.send { //generate lots of tx and save them
        match args.tx_type.as_str() {
            "deposit" => {

                for i in 1 .. args.count {
                    generator.generate_deposit().await?;
                    tracing::info!("processed {} tx", i);
                }

                Ok(())
            }
            _ => Err(load_runner::utils::TestError::GeneratorError(String::from(
                "unknown transaction type",
            ))),
        }
    } else {
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
