use serde::Serialize;

pub mod validate;
pub mod execute;
pub mod mode;
pub mod run;
pub mod response;

pub use mode::ExecutionMode;
pub use execute::execute_action;
pub use validate::validate_config;

/* ---------------- execution output (existing) ---------------- */

#[derive(Debug, Serialize)]
pub struct ExecutionResult {
    pub ok: bool,
    pub runs: u64,
    pub failures: Vec<String>,
    pub max_duration_ms: Option<u128>,
    pub max_memory_kb: Option<u64>,
    pub snapshots_ok: bool,
}

/* ---------------- validation ---------------- */

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Serialize)]
pub struct ValidationError {
    pub code: &'static str,
    pub message: String,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            valid: false,
            errors: vec![ValidationError {
                code,
                message: message.into(),
            }],
        }
    }

    pub fn push_error(&mut self, code: &'static str, message: impl Into<String>) {
        self.valid = false;
        self.errors.push(ValidationError {
            code,
            message: message.into(),
        });
    }

    pub fn is_valid(&self) -> bool {
        self.valid && self.errors.is_empty()
    }
}
