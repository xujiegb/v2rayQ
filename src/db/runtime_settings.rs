use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use turso::{Connection, Row, params};
use uuid::Uuid;

use crate::model::RuntimeSettings;

#[derive(Debug, Error)]
pub enum RuntimeSettingsStoreError {
    #[error(transparent)]
    Database(#[from] turso::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("invalid text column at index {0}")]
    InvalidTextColumn(usize),
    #[error("system clock is before the Unix epoch")]
    InvalidSystemTime,
}

pub async fn save(
    connection: &Connection,
    profile_id: Uuid,
    settings: &RuntimeSettings,
) -> Result<(), RuntimeSettingsStoreError> {
    let config = serde_json::to_string(settings)?;
    let now = unix_timestamp()?;

    connection
        .execute(
            r#"
            INSERT INTO runtime_settings (
                profile_id,
                config,
                updated_at
            )
            VALUES (?1, ?2, ?3)
            ON CONFLICT(profile_id) DO UPDATE SET
                config = excluded.config,
                updated_at = excluded.updated_at
            "#,
            params![
                profile_id.to_string(),
                config,
                now,
            ],
        )
        .await?;

    Ok(())
}

pub async fn get(
    connection: &Connection,
    profile_id: Uuid,
) -> Result<Option<RuntimeSettings>, RuntimeSettingsStoreError> {
    let mut rows = connection
        .query(
            r#"
            SELECT config
            FROM runtime_settings
            WHERE profile_id = ?1
            LIMIT 1
            "#,
            [profile_id.to_string()],
        )
        .await?;

    match rows.next().await? {
        Some(row) => Ok(Some(settings_from_row(&row)?)),
        None => Ok(None),
    }
}

pub async fn get_or_default(
    connection: &Connection,
    profile_id: Uuid,
) -> Result<RuntimeSettings, RuntimeSettingsStoreError> {
    Ok(get(connection, profile_id).await?.unwrap_or_default())
}

pub async fn delete(
    connection: &Connection,
    profile_id: Uuid,
) -> Result<bool, RuntimeSettingsStoreError> {
    let changed = connection
        .execute(
            "DELETE FROM runtime_settings WHERE profile_id = ?1",
            [profile_id.to_string()],
        )
        .await?;

    Ok(changed != 0)
}

fn settings_from_row(
    row: &Row,
) -> Result<RuntimeSettings, RuntimeSettingsStoreError> {
    let config = text(row, 0)?;

    Ok(serde_json::from_str(&config)?)
}

fn text(
    row: &Row,
    index: usize,
) -> Result<String, RuntimeSettingsStoreError> {
    row.get_value(index)?
        .as_text()
        .cloned()
        .ok_or(RuntimeSettingsStoreError::InvalidTextColumn(index))
}

fn unix_timestamp() -> Result<i64, RuntimeSettingsStoreError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| RuntimeSettingsStoreError::InvalidSystemTime)?
        .as_secs();

    Ok(seconds as i64)
}
