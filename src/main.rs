#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _connection = v2rayQ::db::open("v2rayQ.db").await?;

    Ok(())
}
