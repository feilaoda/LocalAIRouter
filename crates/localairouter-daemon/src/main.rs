#[tokio::main]
async fn main() -> localairouter_core::Result<()> {
    localairouter_daemon::run().await
}
