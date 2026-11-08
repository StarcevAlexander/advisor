mod changes;
mod chat;
mod config;
mod httpapi;
mod store;
mod util;
mod zai;

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::OnceCell;

const READ_HEADER_TIMEOUT: Duration = Duration::from_secs(10);
const DATABASE_OPEN_TIMEOUT: Duration = Duration::from_secs(20);

fn main() -> ExitCode {
    let dotenv = config::load_dot_env(config::DOT_ENV_FILE_NAME);
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("ERROR build tokio runtime error={err}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(run(dotenv)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("ERROR server stopped error={err}");
            ExitCode::FAILURE
        }
    }
}

async fn run(dotenv: config::DotEnv) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cfg = config::load(&dotenv).map_err(|err| format!("config: {err}"))?;

    let store_slot: Arc<OnceCell<Arc<dyn store::Store>>> = Arc::new(OnceCell::new());
    init_tracing(store_slot.clone());

    let history: Arc<dyn store::Store> = if cfg.database_url.is_empty() {
        tracing::warn!("DATABASE_URL is not set, history and error log are not stored");
        Arc::new(store::Noop)
    } else {
        let postgres = tokio::time::timeout(
            DATABASE_OPEN_TIMEOUT,
            store::Postgres::open(&cfg.database_url),
        )
        .await
        .map_err(|_| "store: open timeout")??;
        let postgres: Arc<dyn store::Store> = Arc::new(postgres);
        let _ = store_slot.set(postgres.clone());
        postgres
    };

    let change_requester: Option<Arc<dyn changes::Requester>> = if cfg.github_token.is_empty() {
        None
    } else {
        Some(Arc::new(changes::GitHub::new(changes::GitHubConfig {
            token: cfg.github_token.clone(),
            repo: cfg.github_repo.clone(),
            branch: cfg.github_branch.clone(),
            base_url: cfg.github_api_url.clone(),
        })))
    };

    let client = zai::Client::new(&cfg.api_key, &cfg.base_url);
    let responder: Arc<chat::Zai> = Arc::new(chat::Zai::new(
        client,
        cfg.model.clone(),
        cfg.vision_model.clone(),
    ));

    let app = httpapi::router(httpapi::Options {
        responder: responder.clone(),
        planner: responder.clone(),
        changes: change_requester,
        password: cfg.access_password.clone(),
        allowed_origins: cfg.allowed_origins.clone(),
        store: history.clone(),
    });

    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!(addr = %addr, model = %cfg.model, "listening");

    serve(app, addr).await?;
    history.close().await;
    Ok(())
}

fn init_tracing(store_slot: Arc<OnceCell<Arc<dyn store::Store>>>) {
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::prelude::*;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(store::ErrorLogLayer::new(store_slot))
        .init();
}

async fn serve(
    app: axum::Router,
    addr: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
    use hyper_util::server::conn::auto::Builder as HttpBuilder;
    use hyper_util::server::graceful::GracefulShutdown;
    use hyper_util::service::TowerToHyperService;

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let mut http = HttpBuilder::new(TokioExecutor::new());
    http.http1()
        .timer(TokioTimer::default())
        .header_read_timeout(READ_HEADER_TIMEOUT);

    let graceful = GracefulShutdown::new();
    let ctrl_c = tokio::signal::ctrl_c();

    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = match accepted {
                    Ok(pair) => pair,
                    Err(err) => {
                        tracing::warn!(error = %err, "accept failed");
                        continue;
                    }
                };
                let io = TokioIo::new(stream);
                let service = TowerToHyperService::new(app.clone());
                let connection = http.serve_connection(io, service).into_owned();
                let watched = graceful.watch(connection);
                tokio::spawn(async move {
                    if let Err(err) = watched.await {
                        tracing::warn!(error = %err, "connection error");
                    }
                });
            }
            _ = &mut ctrl_c => {
                break;
            }
        }
    }
    drop(listener);
    graceful.shutdown().await;
    Ok(())
}
