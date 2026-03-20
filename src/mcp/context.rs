use crate::use_cases::context::ExecutionTransport;

/// Per-call metadata passed into the MCP service layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpCallContext {
    transport: ExecutionTransport,
}

impl McpCallContext {
    /// Creates a new MCP call context for the specified transport.
    pub const fn new(transport: ExecutionTransport) -> Self {
        Self { transport }
    }

    /// Creates a stdio MCP call context.
    pub const fn stdio() -> Self {
        Self::new(ExecutionTransport::McpStdio)
    }

    /// Creates an HTTP MCP call context.
    pub const fn http() -> Self {
        Self::new(ExecutionTransport::McpHttp)
    }

    /// Returns the originating transport.
    pub const fn transport(self) -> ExecutionTransport {
        self.transport
    }
}
