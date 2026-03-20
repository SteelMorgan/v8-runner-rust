use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcp::error::McpBusinessFailure;

/// High-level status for structured MCP tool payloads.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpToolStatus {
    Success,
    BusinessFailure,
}

/// Structured MCP tool payload returned for successful and business-failure outcomes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct McpToolResult<T> {
    pub status: McpToolStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::mcp::error::McpBusinessError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<T>,
}

impl<T> McpToolResult<T> {
    /// Creates a success-shaped MCP tool payload.
    pub fn success(result: T) -> Self {
        Self {
            status: McpToolStatus::Success,
            result: Some(result),
            error: None,
            response: None,
        }
    }

    /// Creates a business-failure payload preserving the typed response body.
    pub fn business_failure(failure: McpBusinessFailure<T>) -> Self {
        Self {
            status: McpToolStatus::BusinessFailure,
            result: None,
            error: Some(failure.error),
            response: Some(failure.response),
        }
    }
}
