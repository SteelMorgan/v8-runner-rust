/// Identifies the logical command being executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandName {
    Build,
    Test,
    Dump,
    Syntax,
    Launch,
}

impl CommandName {
    /// Returns the stable command label used in logs and CLI envelopes.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Test => "test",
            Self::Dump => "dump",
            Self::Syntax => "syntax",
            Self::Launch => "launch",
        }
    }
}

/// Describes the transport that invoked the use case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionTransport {
    Cli,
    McpStdio,
    McpHttp,
}

/// Per-invocation metadata passed into transport-neutral use cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionContext {
    command: CommandName,
    transport: ExecutionTransport,
}

impl ExecutionContext {
    /// Creates a CLI execution context for the specified command.
    pub const fn cli(command: CommandName) -> Self {
        Self {
            command,
            transport: ExecutionTransport::Cli,
        }
    }

    /// Returns the command being executed.
    pub const fn command(self) -> CommandName {
        self.command
    }

    /// Returns the transport that initiated this execution.
    pub const fn transport(self) -> ExecutionTransport {
        self.transport
    }
}
