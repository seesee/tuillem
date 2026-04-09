use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    println!("tuillem v{}", tuillem_config::version());
    Ok(())
}
