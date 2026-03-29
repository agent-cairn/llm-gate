use thiserror::Error;

#[derive(Debug, Error)]
pub enum GateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Budget not found: {0}")]
    BudgetNotFound(String),
    #[error("Budget exceeded: spent ${spent:.4}, limit ${limit:.4}")]
    BudgetExceeded { spent: f64, limit: f64 },
}
