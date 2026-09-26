pub async fn open(path: &str) -> turso::Result<Connection> {
    let database = Builder::new_local(path).build().await?;
    let connection = database.connect()?;

    connection.execute("PRAGMA foreign_keys = ON", ()).await?;

    Ok(connection)
}
