use gsf_api::server;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let addr: SocketAddr = std::env::var("GSF_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .expect("GSF_BIND_ADDR must be valid socket address");
    server::run_server(addr).await;
}
