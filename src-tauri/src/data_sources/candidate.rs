use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct FieldCandidate<T> {
    pub value: T,
    pub source: &'static str,
    pub observed_at: DateTime<Utc>,
    pub confidence: f32,
}

impl<T> FieldCandidate<T> {
    pub fn new(
        value: T,
        source: &'static str,
        observed_at: DateTime<Utc>,
        confidence: f32,
    ) -> Self {
        Self {
            value,
            source,
            observed_at,
            confidence,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticIssue {
    pub code: String,
    pub source: String,
}

impl DiagnosticIssue {
    pub fn new(code: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            source: source.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AdapterResult<T> {
    pub value: T,
    pub issues: Vec<DiagnosticIssue>,
}

impl<T: Default> Default for AdapterResult<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            issues: Vec::new(),
        }
    }
}
