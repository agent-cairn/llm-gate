use crate::error::GateError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditStatus {
    Ok,
    Warned,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub label: String,
    pub model: String,
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub status: AuditStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl AuditEvent {
    pub fn new(
        label: impl Into<String>,
        model: impl Into<String>,
        provider: impl Into<String>,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        status: AuditStatus,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            label: label.into(),
            model: model.into(),
            provider: provider.into(),
            input_tokens,
            output_tokens,
            cost_usd,
            status,
            request_id: None,
        }
    }
}

pub struct AuditWriter {
    writer: Box<dyn Write + Send>,
}

impl AuditWriter {
    pub fn stdout() -> Self {
        Self {
            writer: Box::new(std::io::stdout()),
        }
    }

    pub fn file(path: &Path) -> Result<Self, GateError> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            writer: Box::new(BufWriter::new(file)),
        })
    }

    pub fn write(&mut self, event: &AuditEvent) -> Result<(), GateError> {
        let line = serde_json::to_string(event)?;
        writeln!(self.writer, "{}", line)?;
        Ok(())
    }
}
