use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::str::FromStr;
use thiserror::Error;

// --- TYPES ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "params")]
pub enum MotionIntent {
    MoveZ { target_nm: f64, velocity_nm_s: f64 },
    MoveXY { target_x_um: f64, target_y_um: f64 },

    ScanSegment {
        vertices: Vec<(f64, f64)>,
        velocity_um_s: f64,
    },

    Hold { duration_ms: u64 },
    EmergencyStop { reason: String },
    JobSegment { job_id: Uuid, segment_index: u32 },
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IntentRecord {
    pub id: String,
    pub created_at: String,
    pub intent_json: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub timestamp: DateTime<Utc>,
    pub z_pos_nm: f64,
    pub x_pos_um: f64,
    pub y_pos_um: f64,
    pub temperature_c: f64,
    pub job_id: Option<Uuid>,
    pub job_progress: f64,
    pub safety_state: String,
}

#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

// --- MANAGER ---

pub struct PersistenceManager {
    pool: SqlitePool,
    last_snapshot: Arc<Mutex<Option<SystemSnapshot>>>,
}

impl PersistenceManager {
    pub async fn new(db_url: &str) -> Result<Self, PersistenceError> {
        let options = SqliteConnectOptions::from_str(db_url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Full);

        let pool = SqlitePool::connect_with(options).await?;

        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS intent_log (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                intent_json TEXT NOT NULL,
                status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                data_json TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_intent_status ON intent_log(status);
        "#).execute(&pool).await?;

        Ok(Self {
            pool,
            last_snapshot: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn log_intent(&self, intent: &MotionIntent) -> Result<String, PersistenceError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let json = serde_json::to_string(intent)?;

        sqlx::query("INSERT INTO intent_log (id, created_at, intent_json, status) VALUES (?, ?, ?, ?)")
            .bind(&id)
            .bind(&now)
            .bind(&json)
            .bind("PENDING")
            .execute(&self.pool)
            .await?;

        Ok(id)
    }

    pub async fn mark_intent_executing(&self, id: &str) -> Result<(), PersistenceError> {
        sqlx::query("UPDATE intent_log SET status = 'EXECUTING' WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_intent_completed(&self, id: &str) -> Result<(), PersistenceError> {
        sqlx::query("UPDATE intent_log SET status = 'COMPLETED' WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn save_snapshot(&self, snap: SystemSnapshot) -> Result<(), PersistenceError> {
        // [FIX] Liveness Hazard: Update In-Memory Cache FIRST
        // This ensures the UI thread (which reads last_snapshot) never waits on slow Disk I/O.
        // The lock is held only for a few nanoseconds to swap the pointer.
        {
            let mut cache = self.last_snapshot.lock().await;
            *cache = Some(snap.clone());
        } // Lock released here

        // Now perform slow IO without holding the lock
        let json = serde_json::to_string(&snap)?;
        let now = snap.timestamp.to_rfc3339();

        sqlx::query("INSERT INTO snapshots (timestamp, data_json) VALUES (?, ?)")
            .bind(now)
            .bind(json)
            .execute(&self.pool)
            .await?;

        // Housekeeping (can be done asynchronously/later, but fine here now that lock is free)
        sqlx::query("DELETE FROM snapshots WHERE id NOT IN (SELECT id FROM snapshots ORDER BY id DESC LIMIT 1000)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn analyze_recovery(&self) -> Result<RecoveryState, PersistenceError> {
        let last_intent = sqlx::query_as::<_, IntentRecord>("SELECT * FROM intent_log ORDER BY created_at DESC LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;

        let last_snap_row: Option<(String, String)> = sqlx::query_as("SELECT timestamp, data_json FROM snapshots ORDER BY id DESC LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;

        let last_snapshot = match last_snap_row {
            Some((_, json)) => Some(serde_json::from_str::<SystemSnapshot>(&json)?),
            None => None,
        };

        match (last_intent, last_snapshot) {
            (None, _) => Ok(RecoveryState::FreshStart),
            (Some(intent), Some(snap)) => {
                if intent.status == "COMPLETED" {
                    Ok(RecoveryState::CleanShutdown(snap))
                } else if intent.status == "EXECUTING" {
                    Ok(RecoveryState::PowerLossDuringMotion {
                        last_safe_snapshot: snap,
                        interrupted_intent: serde_json::from_str(&intent.intent_json)?,
                    })
                } else {
                     Ok(RecoveryState::CleanShutdown(snap))
                }
            },
            _ => Ok(RecoveryState::CorruptedData),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryState {
    FreshStart,
    CleanShutdown(SystemSnapshot),
    PowerLossDuringMotion {
        last_safe_snapshot: SystemSnapshot,
        interrupted_intent: MotionIntent,
    },
    CorruptedData,
}
