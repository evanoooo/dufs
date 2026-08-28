use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Audit operation action categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditAction {
    Upload,
    Delete,
    Move,
    Copy,
    Mkdir,
    Download,
    ZipDownload,
    EditSave,
    AuthFail,
    Login,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditAction::Upload => "UPLOAD",
            AuditAction::Delete => "DELETE",
            AuditAction::Move => "MOVE",
            AuditAction::Copy => "COPY",
            AuditAction::Mkdir => "MKDIR",
            AuditAction::Download => "DOWNLOAD",
            AuditAction::ZipDownload => "ZIP_DOWNLOAD",
            AuditAction::EditSave => "EDIT_SAVE",
            AuditAction::AuthFail => "AUTH_FAIL",
            AuditAction::Login => "LOGIN",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "UPLOAD" => Some(AuditAction::Upload),
            "DELETE" => Some(AuditAction::Delete),
            "MOVE" => Some(AuditAction::Move),
            "COPY" => Some(AuditAction::Copy),
            "MKDIR" => Some(AuditAction::Mkdir),
            "DOWNLOAD" => Some(AuditAction::Download),
            "ZIP_DOWNLOAD" | "ZIP" => Some(AuditAction::ZipDownload),
            "EDIT_SAVE" | "EDIT" => Some(AuditAction::EditSave),
            "AUTH_FAIL" => Some(AuditAction::AuthFail),
            "LOGIN" => Some(AuditAction::Login),
            _ => None,
        }
    }
}

/// Execution status of the audited operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditStatus {
    Success,
    Failed,
}

/// Individual audit record entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: u64,
    pub timestamp: i64,
    pub user: Option<String>,
    pub ip: String,
    pub user_agent: Option<String>,
    pub action: AuditAction,
    pub path: String,
    pub target_path: Option<String>,
    pub status: AuditStatus,
    pub status_code: u16,
    pub size: Option<u64>,
    pub message: Option<String>,
}

/// Filter criteria for querying audit logs
#[derive(Debug, Default, Deserialize)]
pub struct AuditFilter {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub action: Option<String>,
    pub status: Option<String>,
    pub user: Option<String>,
    pub q: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
}

/// Paginated query result
#[derive(Debug, Serialize)]
pub struct AuditQueryResult {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub data: Vec<AuditRecord>,
}

/// Summary metrics and statistics for dashboard
#[derive(Debug, Serialize)]
pub struct AuditStats {
    pub today_total: usize,
    pub today_upload_bytes: u64,
    pub today_download_bytes: u64,
    pub today_alerts: usize,
    pub action_breakdown: HashMap<String, usize>,
}

/// Central audit manager holding ring buffer in memory and appending to optional file
pub struct AuditManager {
    records: Arc<RwLock<VecDeque<AuditRecord>>>,
    counter: Arc<AtomicU64>,
    max_records: usize,
    audit_file: Option<PathBuf>,
}

impl AuditManager {
    pub fn new(max_records: usize, audit_file: Option<PathBuf>) -> Self {
        let max = if max_records == 0 { 10_000 } else { max_records };
        let mut initial_records = VecDeque::new();
        let mut max_id = 0u64;

        if let Some(ref path) = audit_file {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        if let Ok(record) = serde_json::from_str::<AuditRecord>(trimmed) {
                            if record.id > max_id {
                                max_id = record.id;
                            }
                            initial_records.push_back(record);
                            if initial_records.len() > max {
                                initial_records.pop_front();
                            }
                        }
                    }
                }
            }
        }

        Self {
            records: Arc::new(RwLock::new(initial_records)),
            counter: Arc::new(AtomicU64::new(max_id)),
            max_records: max,
            audit_file,
        }
    }

    /// Record an audit event asynchronously
    pub async fn record(&self, mut entry: AuditRecord) {
        let next_id = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
        entry.id = next_id;
        if entry.timestamp == 0 {
            entry.timestamp = Utc::now().timestamp_millis();
        }

        // Persist to file if configured
        if let Some(ref file_path) = self.audit_file {
            if let Ok(json_line) = serde_json::to_string(&entry) {
                let path_clone = file_path.clone();
                tokio::spawn(async move {
                    if let Ok(mut file) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path_clone)
                        .await
                    {
                        let _ = file.write_all(format!("{json_line}\n").as_bytes()).await;
                    }
                });
            }
        }

        let mut lock = self.records.write().await;
        lock.push_back(entry);
        if lock.len() > self.max_records {
            lock.pop_front();
        }
    }

    /// Query audit records with filtering and pagination (newest first)
    pub async fn query(&self, filter: &AuditFilter) -> AuditQueryResult {
        let lock = self.records.read().await;
        let mut matched: Vec<&AuditRecord> = lock
            .iter()
            .rev()
            .filter(|rec| {
                if let Some(ref action_str) = filter.action {
                    if !action_str.is_empty() && action_str != "all" {
                        if let Some(expected_action) = AuditAction::from_str_loose(action_str) {
                            if rec.action != expected_action {
                                return false;
                            }
                        }
                    }
                }

                if let Some(ref status_str) = filter.status {
                    let s = status_str.to_uppercase();
                    if s == "SUCCESS" && rec.status != AuditStatus::Success {
                        return false;
                    } else if s == "FAILED" && rec.status != AuditStatus::Failed {
                        return false;
                    }
                }

                if let Some(ref u) = filter.user {
                    if !u.is_empty() && u != "all" {
                        match &rec.user {
                            Some(user) => {
                                if !user.eq_ignore_ascii_case(u) {
                                    return false;
                                }
                            }
                            None => {
                                if !u.eq_ignore_ascii_case("anonymous") {
                                    return false;
                                }
                            }
                        }
                    }
                }

                if let Some(start) = filter.start_time {
                    if rec.timestamp < start {
                        return false;
                    }
                }

                if let Some(end) = filter.end_time {
                    if rec.timestamp > end {
                        return false;
                    }
                }

                if let Some(ref q) = filter.q {
                    let query_lower = q.to_lowercase();
                    let match_path = rec.path.to_lowercase().contains(&query_lower);
                    let match_target = rec
                        .target_path
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(&query_lower))
                        .unwrap_or(false);
                    let match_ip = rec.ip.contains(&query_lower);
                    let match_user = rec
                        .user
                        .as_ref()
                        .map(|u| u.to_lowercase().contains(&query_lower))
                        .unwrap_or(false);
                    let match_msg = rec
                        .message
                        .as_ref()
                        .map(|m| m.to_lowercase().contains(&query_lower))
                        .unwrap_or(false);

                    if !match_path && !match_target && !match_ip && !match_user && !match_msg {
                        return false;
                    }
                }

                true
            })
            .collect();

        let total = matched.len();
        let page = filter.page.unwrap_or(1).max(1);
        let page_size = filter.page_size.unwrap_or(20).clamp(1, 200);

        let start_index = (page - 1) * page_size;
        let data: Vec<AuditRecord> = if start_index >= total {
            vec![]
        } else {
            matched
                .drain(start_index..std::cmp::min(start_index + page_size, total))
                .cloned()
                .collect()
        };

        AuditQueryResult {
            total,
            page,
            page_size,
            data,
        }
    }

    /// Compute statistics starting from a specific timestamp
    pub async fn stats(&self, since_ms: i64) -> AuditStats {
        let lock = self.records.read().await;
        let mut today_total = 0;
        let mut today_upload_bytes = 0u64;
        let mut today_download_bytes = 0u64;
        let mut today_alerts = 0;
        let mut action_breakdown = HashMap::new();

        for rec in lock.iter() {
            if rec.timestamp >= since_ms {
                today_total += 1;
                *action_breakdown.entry(rec.action.as_str().to_string()).or_insert(0) += 1;

                if rec.action == AuditAction::Upload || rec.action == AuditAction::EditSave {
                    if let Some(sz) = rec.size {
                        today_upload_bytes = today_upload_bytes.saturating_add(sz);
                    }
                } else if rec.action == AuditAction::Download || rec.action == AuditAction::ZipDownload {
                    if let Some(sz) = rec.size {
                        today_download_bytes = today_download_bytes.saturating_add(sz);
                    }
                }

                if rec.action == AuditAction::AuthFail || rec.status == AuditStatus::Failed {
                    today_alerts += 1;
                }
            }
        }

        AuditStats {
            today_total,
            today_upload_bytes,
            today_download_bytes,
            today_alerts,
            action_breakdown,
        }
    }

    /// Export matching records to CSV format
    pub async fn export_csv(&self, filter: &AuditFilter) -> String {
        let mut csv = String::from("ID,Timestamp,Time (UTC),User,IP,Action,Path,Destination,Status,StatusCode,Size(Bytes),Message\n");
        let query_res = self.query(&AuditFilter {
            page: Some(1),
            page_size: Some(self.max_records),
            action: filter.action.clone(),
            status: filter.status.clone(),
            user: filter.user.clone(),
            q: filter.q.clone(),
            start_time: filter.start_time,
            end_time: filter.end_time,
        }).await;

        for r in query_res.data {
            let dt = chrono::DateTime::from_timestamp_millis(r.timestamp)
                .map(|t| t.to_rfc3339())
                .unwrap_or_default();
            let user = r.user.as_deref().unwrap_or("anonymous");
            let target = r.target_path.as_deref().unwrap_or("");
            let status = match r.status {
                AuditStatus::Success => "SUCCESS",
                AuditStatus::Failed => "FAILED",
            };
            let size = r.size.map(|s| s.to_string()).unwrap_or_default();
            let msg = r.message.as_deref().unwrap_or("").replace('"', "\"\"");

            csv.push_str(&format!(
                "{},{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},{},\"{}\"\n",
                r.id,
                r.timestamp,
                dt,
                escape_csv(user),
                escape_csv(&r.ip),
                r.action.as_str(),
                escape_csv(&r.path),
                escape_csv(target),
                status,
                r.status_code,
                size,
                msg
            ));
        }

        csv
    }

    /// Clear all records in memory and truncate log file if configured
    pub async fn clear(&self) {
        let mut lock = self.records.write().await;
        lock.clear();
        self.counter.store(0, Ordering::SeqCst);

        if let Some(ref path) = self.audit_file {
            let _ = std::fs::write(path, "");
        }
    }
}

fn escape_csv(s: &str) -> String {
    s.replace('"', "\"\"")
}
