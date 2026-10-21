use anyhow::Result;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cancel = CancellationToken::new();
    let mut task = tokio::spawn(coden_server::run(cancel.clone()));

    tokio::select! {
        res = &mut task => {
            res??;
        }
        _ = tokio::signal::ctrl_c() => {
            cancel.cancel();
            task.await??;
        }
    }

    Ok(())
}
