pub mod nodes;
pub mod profiles;
pub mod schema;
pub mod subscriptions;

use turso::{Builder, Connection};

pub async fn open(path: &str) -> turso::Result<Connection> {
    let database = Builder::new_local(path).build().await?;
    let connection = database.connect()?;

    connection.execute("PRAGMA foreign_keys = ON", ()).await?;
    schema::initialize(&connection).await?;

    Ok(connection)
}
