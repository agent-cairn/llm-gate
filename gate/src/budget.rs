use crate::error::GateError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BudgetAction {
    Block,
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub label: String,
    pub limit_usd: f64,
    pub spent_usd: f64,
    pub action: BudgetAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpendResult {
    Ok,
    Warned { spent: f64, limit: f64 },
    Blocked { spent: f64, limit: f64 },
}

#[derive(Debug, Default)]
pub struct BudgetStore {
    budgets: HashMap<String, Budget>,
    path: Option<PathBuf>,
}

impl BudgetStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(path: &Path) -> Result<Self, GateError> {
        let content = std::fs::read_to_string(path)?;
        let budgets: HashMap<String, Budget> = serde_json::from_str(&content)?;
        Ok(Self {
            budgets,
            path: Some(path.to_owned()),
        })
    }

    pub fn save(&self) -> Result<(), GateError> {
        if let Some(path) = &self.path {
            let content = serde_json::to_string_pretty(&self.budgets)?;
            std::fs::write(path, content)?;
        }
        Ok(())
    }

    pub fn add_budget(
        &mut self,
        label: impl Into<String>,
        limit_usd: f64,
        action: BudgetAction,
    ) -> Result<(), GateError> {
        let label = label.into();
        self.budgets.insert(
            label.clone(),
            Budget {
                label,
                limit_usd,
                spent_usd: 0.0,
                action,
            },
        );
        self.save()
    }

    pub fn record_spend(&mut self, label: &str, cost_usd: f64) -> Result<SpendResult, GateError> {
        let budget = self
            .budgets
            .get_mut(label)
            .ok_or_else(|| GateError::BudgetNotFound(label.to_string()))?;
        budget.spent_usd += cost_usd;
        let spent = budget.spent_usd;
        let limit = budget.limit_usd;
        let result = if spent >= limit {
            match budget.action {
                BudgetAction::Block => SpendResult::Blocked { spent, limit },
                BudgetAction::Warn => SpendResult::Warned { spent, limit },
            }
        } else {
            SpendResult::Ok
        };
        self.save()?;
        Ok(result)
    }

    pub fn get(&self, label: &str) -> Option<&Budget> {
        self.budgets.get(label)
    }

    pub fn reset(&mut self, label: &str) -> Result<(), GateError> {
        let budget = self
            .budgets
            .get_mut(label)
            .ok_or_else(|| GateError::BudgetNotFound(label.to_string()))?;
        budget.spent_usd = 0.0;
        self.save()
    }

    pub fn all(&self) -> impl Iterator<Item = &Budget> {
        self.budgets.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_record_spend_ok() {
        let mut store = BudgetStore::new();
        store.add_budget("test", 10.0, BudgetAction::Block).unwrap();
        let result = store.record_spend("test", 1.0).unwrap();
        assert_eq!(result, SpendResult::Ok);
    }

    #[test]
    fn test_budget_block_when_exceeded() {
        let mut store = BudgetStore::new();
        store.add_budget("test", 1.0, BudgetAction::Block).unwrap();
        store.record_spend("test", 0.5).unwrap();
        let r2 = store.record_spend("test", 0.6).unwrap();
        assert!(matches!(r2, SpendResult::Blocked { .. }));
    }

    #[test]
    fn test_budget_warn_when_exceeded() {
        let mut store = BudgetStore::new();
        store.add_budget("warn", 1.0, BudgetAction::Warn).unwrap();
        let r = store.record_spend("warn", 1.5).unwrap();
        assert!(matches!(r, SpendResult::Warned { .. }));
    }

    #[test]
    fn test_reset_clears_spent() {
        let mut store = BudgetStore::new();
        store.add_budget("r", 5.0, BudgetAction::Block).unwrap();
        store.record_spend("r", 3.0).unwrap();
        store.reset("r").unwrap();
        assert_eq!(store.get("r").unwrap().spent_usd, 0.0);
    }

    #[test]
    fn test_missing_budget_returns_error() {
        let mut store = BudgetStore::new();
        assert!(store.record_spend("nonexistent", 1.0).is_err());
    }
}
