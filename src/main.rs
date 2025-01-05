#[tokio::main]
async fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let _ = server::main().await;
}
