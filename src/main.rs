use async_executors::TokioTp;
use language_server::{LanguageService, LoggingMiddleware};
use log::LevelFilter;
use std::{convert::TryFrom, env, error, fs::OpenOptions, path::PathBuf, sync::Arc};
use structopt::StructOpt;
use texlab::{
    server::{LatexLanguageServer, LatexLanguageServerParams},
    tex::Distribution,
};
use tokio_util::compat::*;

/// An implementation of the Language Server Protocol for LaTeX
#[derive(Debug, StructOpt)]
struct Opts {
    /// Increase message verbosity (-vvvv for max verbosity)
    #[structopt(short, long, parse(from_occurrences))]
    verbosity: u8,

    /// No output printed to stderr
    #[structopt(short, long)]
    quiet: bool,

    /// Write the logging output to FILE
    #[structopt(long, name = "FILE", parse(from_os_str))]
    log_file: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let opts = Opts::from_args();
    setup_logger(opts);

    let executor = TokioTp::try_from(tokio::runtime::Builder::new().enable_all())
        .expect("failed to create thread pool");

    let current_dir = env::current_dir().expect("failed to get working directory");

    executor.clone().block_on(async move {
        let server = Arc::new(LatexLanguageServer::new(
            LatexLanguageServerParams::builder()
                .executor(executor.clone())
                .distro(Distribution::detect().await)
                .current_dir(Arc::new(current_dir))
                .build(),
        ));

        LanguageService::builder()
            .server(Arc::clone(&server))
            .input(tokio::io::stdin().compat())
            .output(tokio::io::stdout().compat_write())
            .executor(executor)
            .middlewares(vec![server, Arc::new(LoggingMiddleware)])
            .build()
            .listen()
            .await;
    });

    Ok(())
}

fn setup_logger(opts: Opts) {
    let verbosity_level = if !opts.quiet {
        match opts.verbosity {
            0 => LevelFilter::Error,
            1 => LevelFilter::Warn,
            2 => LevelFilter::Info,
            3 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        }
    } else {
        LevelFilter::Off
    };

    let logger = fern::Dispatch::new()
        .format(|out, message, record| out.finish(format_args!("{} - {}", record.level(), message)))
        .level(verbosity_level)
        .filter(|metadata| {
            metadata.target().contains("language_server") || metadata.target().contains("texlab")
        })
        .chain(std::io::stderr());

    let logger = match opts.log_file {
        Some(log_file) => logger.chain(
            OpenOptions::new()
                .write(true)
                .create(true)
                .open(log_file)
                .expect("failed to open log file"),
        ),
        None => logger,
    };

    logger.apply().expect("failed to initialize logger");
}
