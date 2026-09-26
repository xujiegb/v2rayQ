
    
  
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use turso::{Connection, Row, params};
use uuid::Uuid;

use crate::model::Profile;

#[derive(Debug, Error)]
pub enum ProfileStoreError {
    #[error(transparent)]
    Database(#[from] turso::Error),
    #[error(transparent)]
    Uuid(#[from] uuid::Error),
    #[error("invalid text column at index {0}")]
    InvalidTextColumn(usize),
    #[error("system clock is before the Unix epoch")]
    InvalidSystemTime,
}

pub async fn save(
    connection: &Connection,
    profile: &Profile,
) -> Result<(), ProfileStoreError> {
    let now = unix_timestamp()?;

    connection
        .execute(
            r#"
            INSERT INTO profiles (
                id,
                name,
                active_node_id,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?4)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                active_node_id = excluded.active_node_id,
                updated_at = excluded.updated_at
            "#,
            params![
                profile.id.to_string(),
                profile.name.as_str(),
                profile.active_node_id.map(|id| id.to_string()),
                now,
            ],
        )
        .await?;

    Ok(())
}

pub async fn get(
    connection: &Connection,
    id: Uuid,
) -> Result<Option<Profile>, ProfileStoreError> {
    let mut rows = connection
        .query(
            r#"
            SELECT id, name, active_node_id
            FROM profiles
            WHERE id = ?1
            LIMIT 1
            "#,
            [id.to_string()],
        )
        .await?;

    match rows.next().await? {
        Some(row) => Ok(Some(profile_from_row(&row)?)),
        None => Ok(None),
    }
}

pub async fn list(
    connection: &Connection,
) -> Result<Vec<Profile>, ProfileStoreError> {
    let mut rows = connection
        .query(
            r#"
            SELECT id, name, active_node_id
            FROM profiles
            ORDER BY created_at ASC, id ASC
            "#,
            (),
        )
        .await?;

    let mut profiles = Vec::new();

    while let Some(row) = rows.next().await? {
        profiles.push(profile_from_row(&row)?);
    }

