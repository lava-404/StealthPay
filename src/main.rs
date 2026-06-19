use axum::{
    routing::{get, post},
    Router,
};
use solana_address::Address;
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
mod relayer;
mod routes;
mod stealth;

// Shared state to pass configuration and clients into our route handlers
pub struct AppState {
    pub rpc_client: RpcClient,
    pub relayer_config: relayer::validate::RelayerConfig,
}

#[tokio::main]
async fn main() {
    println!("Starting Stealth Relayer Node...");

    let rpc_client = RpcClient::new("https://api.devnet.solana.com".to_string());

    let relayer_config = relayer::validate::RelayerConfig {
        relayer_fee_payer_keypair: Keypair::new(),
        relayer_token_account: Address::new_unique(),
        min_service_fee_usdc: 1_000_000,
    };

    let shared_state = Arc::new(AppState {
        rpc_client,
        relayer_config,
    });

    let app = Router::new()
        .route("/meta-address/create", post(routes::meta::create_meta_address))
        .route("/send", post(routes::send::send_stealth_payment))
        .route("/relay", post(routes::relay::relay_transaction))
        .route("/payments", get(routes::payments::get_payments))
        .route("/health", get(routes::health::health))
        .layer(CorsLayer::permissive())
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Server running on http://0.0.0.0:8080");

    axum::serve(listener, app).await.unwrap();
}
