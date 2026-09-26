use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use turso::{Connection, Row, params};
use uuid::Uuid;

use crate::model::{Endpoint, Node, Protocol};

#[derive(Debug, Error)]
pub enum NodeStoreError {
    #[error(transparent)]
    Database(#[from] turso::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Uuid(#[from] uuid::Error),
    #[error("invalid text column at index {0}")]
    InvalidTextColumn(usize),
    #[error("system clock is before the Unix epoch")]
    InvalidSystemTime,
}

pub async fn save(
    connection: &Connection,
    node: &Node,
    subscription_id: Option<&str>,
) -> Result<(), NodeStoreError> {
    let endpoint = serde_json::to_string(&node.endpoint)?;
    let config = serde_json::to_string(&node.protocol)?;
    let protocol = protocol_name(&node.protocol);
    let now = unix_timestamp()?;

    connection
        .execute(
            r#"
            INSERT INTO nodes (
                id,
                subscription_id,
                name,
                protocol,
                endpoint,
                config,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
            ON CONFLICT(id) DO UPDATE SET
                subscription_id = excluded.subscription_id,
                name = excluded.name,
                protocol = excluded.protocol,
                endpoint = excluded.endpoint,
                config = excluded.config,
                updated_at = excluded.updated_at
            "#,
            params![
                node.id.to_string(),
                subscription_id,
                node.name.as_str(),
                protocol,
                endpoint,
                config,
                now,
            ],
        )
        .await?;

    Ok(())
}

pub async fn get(
    connection: &Connection,
    id: Uuid,
) -> Result<Option<Node>, NodeStoreError> {
    let mut rows = connection
        .query(
            r#"
            SELECT id, name, endpoint, config
            FROM nodes
            WHERE id = ?1
            LIMIT 1
            "#,
            [id.to_string()],
        )
        .await?;

    match rows.next().await? {
        Some(row) => Ok(Some(node_from_row(&row)?)),
        None => Ok(None),
    }
}

pub async fn list(connection: &Connection) -> Result<Vec<Node>, NodeStoreError> {
    let mut rows = connection
        .query(
            r#"
            SELECT id, name, endpoint, config
            FROM nodes
            ORDER BY created_at ASC, id ASC
            "#,
            (),
        )
        .await?;

    let mut nodes = Vec::new();

    while let Some(row) = rows.next().await? {
        nodes.push(node_from_row(&row)?);
    }

    Ok(nodes)
}

pub async fn list_by_protocol(
    connection: &Connection,
    protocol: &str,
) -> Result<Vec<Node>, NodeStoreError> {
    let mut rows = connection
        .query(
            r#"
            SELECT id, name, endpoint, config
            FROM nodes
            WHERE protocol = ?1
            ORDER BY created_at ASC, id ASC
            "#,
            [protocol],
        )
        .await?;

    let mut nodes = Vec::new();

    while let Some(row) = rows.next().await? {
        nodes.push(node_from_row(&row)?);
    }

    Ok(nodes)
}

pub async fn delete(
    connection: &Connection,
    id: Uuid,
) -> Result<bool, NodeStoreError> {
    let changed = connection
        .execute(
            "DELETE FROM nodes WHERE id = ?1",
            [id.to_string()],
        )
        .await?;

    Ok(changed != 0)
}

fn node_from_row(row: &Row) -> Result<Node, NodeStoreError> {
    let id = Uuid::parse_str(&text(row, 0)?)?;
    let name = text(row, 1)?;
    let endpoint = serde_json::from_str::<Endpoint>(&text(row, 2)?)?;
    let protocol = serde_json::from_str::<Protocol>(&text(row, 3)?)?;

    Ok(Node {
        id,
        name,
        endpoint,
        protocol,
    })
}

fn text(row: &Row, index: usize) -> Result<String, NodeStoreError> {
    row.get_value(index)?
        .as_text()
        .cloned()
        .ok_or(NodeStoreError::InvalidTextColumn(index))
}

fn protocol_name(protocol: &Protocol) -> &'static str {
    match protocol {
        Protocol::Vless(_) => "vless",
        Protocol::Vmess(_) => "vmess",
        Protocol::Trojan(_) => "trojan",
        Protocol::Shadowsocks(_) => "shadowsocks",
        Protocol::AnyTls(_) => "anytls",
        Protocol::Hysteria2(_) => "hysteria2",
    }
}

fn unix_timestamp() -> Result<i64, NodeStoreError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| NodeStoreError::InvalidSystemTime)?
        .as_secs();

    Ok(seconds as i64)
}
