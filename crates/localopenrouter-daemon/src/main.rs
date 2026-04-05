#[tokio::main]
async fn main() -> localopenrouter_core::Result<()> {
    localopenrouter_daemon::run().await
}
