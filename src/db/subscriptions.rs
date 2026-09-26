use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use turso::{Connection, Row, params};
use uuid::Uuid;

use crate::model::Subscription;

#[derive(Debug, Error)]
pub enum SubscriptionStoreError {
    #[error(transparent)]
    Database(#[from] turso::Error),
    #[error(transparent)]
    Uuid(#[from] uuid::Error),
    #[error("invalid text column at index {0}")]
    InvalidTextColumn(usize),
    #[error("invalid integer column at index {0}")]
    InvalidIntegerColumn(usize),
    #[error("system clock is before the Unix epoch")]
    InvalidSystemTime,
}

pub async fn save(
    connection: &Connection,
    subscription: &Subscription,
) -> Result<(), SubscriptionStoreError> {
    let now = unix_timestamp()?;

    connection
        .execute(
            r#"
            INSERT INTO subscriptions (
                id,
                name,
                url,
                enabled,
                created_at,
                updated_at,
                last_updated_at,
                last_error
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                url = excluded.url,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at,
                last_updated_at = excluded.last_updated_at,
                last_error = excluded.last_error
            "#,
            params![
                subscription.id.to_string(),
                subscription.name.as_str(),
                subscription.url.as_str(),
                subscription.enabled as i64,
                now,
                subscription.last_updated_at,
                subscription.last_error.as_deref(),
            ],
        )
        .await?;

    Ok(())
}

pub async fn get(
    connection: &Connection,
    id: Uuid,
) -> Result<Option<Subscription>, SubscriptionStoreError> {
    let mut rows = connection
        .query(
            r#"
            SELECT id, name, url, enabled, last_updated_at, last_error
            FROM subscriptions
            WHERE id = ?1
            LIMIT 1
            "#,
            [id.to_string()],
        )
        .await?;

    match rows.next().await? {
        Some(row) => Ok(Some(subscription_from_row(&row)?)),
        None => Ok(None),
    }
}

pub async fn list(
    connection: &Connection,
) -> Result<Vec<Subscription>, SubscriptionStoreError> {
    let mut rows = connection
        .query(
            r#"
            SELECT id, name, url, enabled, last_updated_at, last_error
            FROM subscriptions
            ORDER BY created_at ASC, id ASC
            "#,
            (),
        )
        .await?;

    let mut subscriptions = Vec::new();

    while let Some(row) = rows.next().await? {
        subscriptions.push(subscription_from_row(&row)?);
    }

    Ok(subscriptions)
}

pub async fn set_enabled(
    connection: &Connection,
    id: Uuid,
    enabled: bool,
) -> Result<bool, SubscriptionStoreError> {
    let now = unix_timestamp()?;

    let changed = connection
        .execute(
            r#"
            UPDATE subscriptions
            SET enabled = ?2, updated_at = ?3
            WHERE id = ?1
            "#,
            params![id.to_string(), enabled as i64, now],
        )
        .await?;

    Ok(changed != 0)
}

pub async fn set_refresh_result(
    connection: &Connection,
    id: Uuid,
    error: Option<&str>,
) -> Result<bool, SubscriptionStoreError> {
    let now = unix_timestamp()?;

    let changed = connection
        .execute(
            r#"
            UPDATE subscriptions
            SET
                last_updated_at = ?2,
                last_error = ?3,
                updated_at = ?2
            WHERE id = ?1
            "#,
            params![id.to_string(), now, error],
        )
        .await?;

    Ok(changed != 0)
}

pub async fn delete(
    connection: &Connection,
    id: Uuid,
) -> Result<bool, SubscriptionStoreError> {
    let changed = connection
        .execute(
            "DELETE FROM subscriptions WHERE id = ?1",
            [id.to_string()],
        )
        .await?;

    Ok(changed != 0)
}

fn subscription_from_row(
    row: &Row,
) -> Result<Subscription, SubscriptionStoreError> {
    Ok(Subscription {
        id: Uuid::parse_str(&text(row, 0)?)?,
        name: text(row, 1)?,
        url: text(row, 2)?,
        enabled: integer(row, 3)? != 0,
        last_updated_at: optional_integer(row, 4)?,
        last_error: optional_text(row, 5)?,
    })
}

fn text(row: &Row, index: usize) -> Result<String, SubscriptionStoreError> {
    row.get_value(index)?
        .as_text()
        .cloned()
        .ok_or(SubscriptionStoreError::InvalidTextColumn(index))
}

fn optional_text(
    row: &Row,
    index: usize,
) -> Result<Option<String>, SubscriptionStoreError> {
    let value = row.get_value(index)?;

    if value.is_null() {
        return Ok(None);
    }

    value
        .as_text()
        .cloned()
        .map(Some)
        .ok_or(SubscriptionStoreError::InvalidTextColumn(index))
}

fn integer(row: &Row, index: usize) -> Result<i64, SubscriptionStoreError> {
    row.get_value(index)?
        .as_integer()
        .ok_or(SubscriptionStoreError::InvalidIntegerColumn(index))
}

fn optional_integer(
    row: &Row,
    index: usize,
) -> Result<Option<i64>, SubscriptionStoreError> {
    let value = row.get_value(index)?;

    if value.is_null() {
        return Ok(None);
    }

    value
        .as_integer()
        .map(Some)
        .ok_or(SubscriptionStoreError::InvalidIntegerColumn(index))
}

fn unix_timestamp() -> Result<i64, SubscriptionStoreError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SubscriptionStoreError::InvalidSystemTime)?
        .as_secs();

    Ok(seconds as i64)
}
