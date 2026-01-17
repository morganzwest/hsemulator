use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Validate,
    Execute,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        ExecutionMode::Execute
    }
}
