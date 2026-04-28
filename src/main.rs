use clap::Parser;

mod cli;
mod complete;
mod config;
mod error;
mod git;
mod github;
mod paths;
mod repo_setup;
mod resolve;
mod shell;
mod source;
mod template;
mod ui;
mod workspace;

use error::PowError;

fn main() {
    if let Err(e) = color_eyre::install() {
        eprintln!("failed to install color-eyre: {e}");
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("POW_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .without_time()
        .init();

    let cli = cli::Cli::parse();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    let result: Result<(), PowError> = rt.block_on(cli::dispatch(cli));

    if let Err(err) = result {
        let code = err.exit_code();
        eprintln!("error: {err}");
        std::process::exit(code);
    }
}
