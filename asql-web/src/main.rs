use std::net::SocketAddr;

use asql_backend::AppConfig;
use axum::Router;
use clap::Parser;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

mod api;

type AppState = asql_backend::BackendHandle;

#[derive(Parser)]
#[command(name = "asql-web", about = "aSqlDB Web Server")]
struct Cli {
    /// Port to listen on (0 = random available port)
    #[arg(short, long, env = "ASQL_PORT", default_value_t = 5580)]
    port: u16,

    /// Config directory path
    #[arg(short = 'c', long = "config-dir", env = "ASQL_CONFIG_DIR")]
    config_dir: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let static_dir = std::env::var("ASQL_STATIC_DIR")
        .unwrap_or_else(|_| "frontend/dist".to_string());

    let config_dir = cli
        .config_dir
        .map(std::path::PathBuf::from)
        .unwrap_or_else(AppConfig::default_dir);

    let backend = asql_backend::BackendHandle::new(config_dir);
    backend.load_connections().await;

    let serve_dir = ServeDir::new(std::path::PathBuf::from(&static_dir))
        .append_index_html_on_directories(true);

    let app = Router::<AppState>::new()
        .nest("/api", api::build_router())
        .with_state(backend)
        .fallback_service(serve_dir);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = TcpListener::bind(addr).await.unwrap();
    let actual_port = listener.local_addr().unwrap().port();

    if cli.port == 0 {
        println!("{}", actual_port);
    }

    tracing::info!("Starting asql-web on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    use tokio::signal;
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("Ctrl+C received"); }
        _ = terminate => { tracing::info!("Terminate received"); }
    };

    tracing::info!("Shutting down gracefully...");
}
