use std::sync::Arc;
use std::time::Duration;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData, ServerHandler, ServiceExt,
};
use thiserror::Error;
use tokio::sync::Semaphore;

use crate::config::model::AppConfig;
use crate::mcp::context::McpCallContext;
use crate::mcp::error::{McpInternalError, McpServiceResult};
use crate::mcp::port::{DefaultMcpUseCasePort, McpUseCasePort};
use crate::mcp::request::{
    McpBuildProjectRequest, McpCheckSyntaxDesignerConfigRequest,
    McpCheckSyntaxDesignerModulesRequest, McpCheckSyntaxEdtRequest, McpDumpConfigRequest,
    McpLaunchAppRequest, McpRunAllTestsRequest, McpRunModuleTestsRequest,
};
use crate::mcp::service::McpService;
use crate::mcp::tool_result::McpToolResult;

type SharedMcpUseCasePort = Arc<dyn McpUseCasePort + Send + Sync>;

/// Bootstrap errors returned by the MCP stdio server.
#[derive(Debug, Error)]
pub enum McpServerError {
    #[error("failed to build tokio runtime for MCP stdio: {0}")]
    BuildRuntime(std::io::Error),

    #[error("failed to start MCP stdio server: {0}")]
    Start(String),

    #[error("MCP stdio server task failed: {0}")]
    Task(String),
}

/// Runs the MCP stdio server until the transport closes.
pub fn serve_stdio(config: AppConfig) -> Result<(), McpServerError> {
    let shutdown_timeout = shutdown_grace_period(&config);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("v8tr-mcp")
        .build()
        .map_err(McpServerError::BuildRuntime)?;

    let result = runtime.block_on(async move {
        let server = McpStdioServer::new(Arc::new(config));
        let running = server
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|error| McpServerError::Start(error.to_string()))?;

        running
            .waiting()
            .await
            .map_err(|error| McpServerError::Task(error.to_string()))?;
        Ok(())
    });

    runtime.shutdown_timeout(shutdown_timeout);
    result
}

/// rmcp-backed stdio transport adapter over the MCP service layer.
#[derive(Clone)]
pub struct McpStdioServer {
    config: Arc<AppConfig>,
    port: SharedMcpUseCasePort,
    concurrency_limit: Arc<Semaphore>,
    tool_router: ToolRouter<Self>,
}

impl McpStdioServer {
    /// Creates a stdio server using the production use-case port.
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self::with_port(config, Arc::new(DefaultMcpUseCasePort))
    }

    /// Creates a stdio server with an injected MCP use-case port.
    pub fn with_port(config: Arc<AppConfig>, port: SharedMcpUseCasePort) -> Self {
        Self {
            concurrency_limit: Arc::new(Semaphore::new(max_concurrent_calls(config.as_ref()))),
            config,
            port,
            tool_router: Self::tool_router(),
        }
    }

    async fn execute_tool<TRequest, TResponse>(
        &self,
        request: TRequest,
        method: impl FnOnce(Arc<AppConfig>, SharedMcpUseCasePort, TRequest) -> McpServiceResult<TResponse>
            + Send
            + 'static,
    ) -> Result<CallToolResult, ErrorData>
    where
        TRequest: Send + 'static,
        TResponse: serde::Serialize + Send + 'static,
    {
        let _permit = self
            .concurrency_limit
            .clone()
            .acquire_owned()
            .await
            .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
        let config = self.config.clone();
        let port = self.port.clone();
        let result = tokio::task::spawn_blocking(move || method(config, port, request))
            .await
            .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

        match result {
            Ok(response) => Ok(CallToolResult::structured(
                serde_json::to_value(McpToolResult::success(response))
                    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?,
            )),
            Err(crate::mcp::error::McpServiceError::Business(failure)) => {
                Ok(CallToolResult::structured_error(
                    serde_json::to_value(McpToolResult::business_failure(failure))
                        .map_err(|error| ErrorData::internal_error(error.to_string(), None))?,
                ))
            }
            Err(crate::mcp::error::McpServiceError::Internal(error)) => {
                Err(internal_error_to_mcp(error))
            }
        }
    }
}

#[tool_router(router = tool_router)]
impl McpStdioServer {
    #[tool(description = "Run all tests")]
    async fn run_all_tests(
        &self,
        Parameters(request): Parameters<McpRunAllTestsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.execute_tool(request, |config, port, request| {
            let service = McpService::with_port(config.as_ref(), port);
            service.run_all_tests(McpCallContext::stdio(), &request)
        })
        .await
    }

    #[tool(description = "Run tests for a specific module")]
    async fn run_module_tests(
        &self,
        Parameters(request): Parameters<McpRunModuleTestsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.execute_tool(request, |config, port, request| {
            let service = McpService::with_port(config.as_ref(), port);
            service.run_module_tests(McpCallContext::stdio(), &request)
        })
        .await
    }

    #[tool(description = "Build the project")]
    async fn build_project(
        &self,
        Parameters(request): Parameters<McpBuildProjectRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.execute_tool(request, |config, port, request| {
            let service = McpService::with_port(config.as_ref(), port);
            service.build_project(McpCallContext::stdio(), &request)
        })
        .await
    }

    #[tool(description = "Dump configuration to files")]
    async fn dump_config(
        &self,
        Parameters(request): Parameters<McpDumpConfigRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.execute_tool(request, |config, port, request| {
            let service = McpService::with_port(config.as_ref(), port);
            service.dump_config(McpCallContext::stdio(), &request)
        })
        .await
    }

    #[tool(description = "Launch a 1C application")]
    async fn launch_app(
        &self,
        Parameters(request): Parameters<McpLaunchAppRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.execute_tool(request, |config, port, request| {
            let service = McpService::with_port(config.as_ref(), port);
            service.launch_app(McpCallContext::stdio(), &request)
        })
        .await
    }

    #[tool(description = "Run EDT syntax check")]
    async fn check_syntax_edt(
        &self,
        Parameters(request): Parameters<McpCheckSyntaxEdtRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.execute_tool(request, |config, port, request| {
            let service = McpService::with_port(config.as_ref(), port);
            service.check_syntax_edt(McpCallContext::stdio(), &request)
        })
        .await
    }

    #[tool(description = "Run Designer configuration syntax check")]
    async fn check_syntax_designer_config(
        &self,
        Parameters(request): Parameters<McpCheckSyntaxDesignerConfigRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.execute_tool(request, |config, port, request| {
            let service = McpService::with_port(config.as_ref(), port);
            service.check_syntax_designer_config(McpCallContext::stdio(), &request)
        })
        .await
    }

    #[tool(description = "Run Designer modules syntax check")]
    async fn check_syntax_designer_modules(
        &self,
        Parameters(request): Parameters<McpCheckSyntaxDesignerModulesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.execute_tool(request, |config, port, request| {
            let service = McpService::with_port(config.as_ref(), port);
            service.check_syntax_designer_modules(McpCallContext::stdio(), &request)
        })
        .await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpStdioServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

fn internal_error_to_mcp(error: McpInternalError) -> ErrorData {
    ErrorData::internal_error(error.message, None)
}

fn max_concurrent_calls(config: &AppConfig) -> usize {
    config.mcp.execution.max_concurrent_calls.max(1)
}

fn shutdown_grace_period(config: &AppConfig) -> Duration {
    Duration::from_secs(config.mcp.execution.shutdown_grace_period_secs.max(1))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use super::{max_concurrent_calls, shutdown_grace_period, McpStdioServer};
    use crate::config::model::{
        AppConfig, BuildConfig, BuilderBackend, McpConfig, McpExecutionConfig, McpHttpConfig,
        PlatformToolConfig, SourceFormat, SourceSetConfig, SourceSetPurpose, TestsConfig,
        ToolsConfig,
    };
    use crate::mcp::port::DefaultMcpUseCasePort;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn execute_tool_respects_configured_concurrency_limit() {
        let server =
            McpStdioServer::with_port(Arc::new(test_config(1, 9)), Arc::new(DefaultMcpUseCasePort));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));

        let first = tokio::spawn(run_probe_call(
            server.clone(),
            active.clone(),
            max_active.clone(),
        ));
        let second = tokio::spawn(run_probe_call(server, active, max_active.clone()));

        first.await.expect("first task join").expect("first call");
        second
            .await
            .expect("second task join")
            .expect("second call");

        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn shutdown_grace_period_uses_configured_value() {
        let config = test_config(3, 42);

        assert_eq!(max_concurrent_calls(&config), 3);
        assert_eq!(shutdown_grace_period(&config), Duration::from_secs(42));
    }

    async fn run_probe_call(
        server: McpStdioServer,
        active: Arc<AtomicUsize>,
        max_active: Arc<AtomicUsize>,
    ) -> Result<(), rmcp::ErrorData> {
        server
            .execute_tool((), move |_, _, ()| {
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(50));
                active.fetch_sub(1, Ordering::SeqCst);
                Ok(String::from("ok"))
            })
            .await
            .map(|_| ())
    }

    fn test_config(max_concurrent_calls: usize, shutdown_grace_period_secs: u64) -> AppConfig {
        AppConfig {
            base_path: PathBuf::from("/tmp/project"),
            work_path: PathBuf::from("/tmp/work"),
            format: SourceFormat::Designer,
            builder: BuilderBackend::Designer,
            connection: String::from("File=/tmp/ib"),
            credentials: Default::default(),
            source_sets: vec![SourceSetConfig {
                name: String::from("main"),
                purpose: SourceSetPurpose::Configuration,
                path: PathBuf::from("."),
            }],
            build: BuildConfig::default(),
            tools: ToolsConfig {
                platform: PlatformToolConfig::default(),
                edt_cli: Default::default(),
            },
            mcp: McpConfig {
                http: McpHttpConfig::default(),
                execution: McpExecutionConfig {
                    max_concurrent_calls,
                    shutdown_grace_period_secs,
                },
            },
            tests: TestsConfig::default(),
        }
    }
}
