use std::path::Path;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tracing::info;

pub async fn init_db(db_path: &Path) -> anyhow::Result<SqlitePool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // AUDIT FIX: Enable WAL mode for corruption resistance
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(&pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            gdsii_path TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )"
    )
    .execute(&pool)
    .await?;

    info!("Database initialized at {:?}", db_path);
    Ok(pool)
}
