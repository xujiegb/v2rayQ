use turso::Connection;

const STATEMENTS: &[&str] = &[
    r#"
    CREATE TABLE IF NOT EXISTS subscriptions (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        url TEXT NOT NULL,
        enabled INTEGER NOT NULL DEFAULT 1,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        last_updated_at INTEGER,
        last_error TEXT
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS nodes (
        id TEXT PRIMARY KEY NOT NULL,
        subscription_id TEXT,
        name TEXT NOT NULL,
        protocol TEXT NOT NULL,
        endpoint TEXT NOT NULL,
        config TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS profiles (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        active_node_id TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        FOREIGN KEY (active_node_id) REFERENCES nodes(id) ON DELETE SET NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS runtime_settings (
        profile_id TEXT PRIMARY KEY NOT NULL,
        config TEXT NOT NULL,
        updated_at INTEGER NOT NULL,
        FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_nodes_subscription_id
    ON nodes(subscription_id)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_nodes_protocol
    ON nodes(protocol)
    "#,
];

pub async fn initialize(connection: &Connection) -> turso::Result<()> {
    for statement in STATEMENTS {
        connection.execute(statement, ()).await?;
    }

    Ok(())
}
