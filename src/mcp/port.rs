use crate::config::model::AppConfig;
use crate::domain::build::BuildResult;
use crate::domain::dump::DumpResult;
use crate::domain::launch::LaunchResult;
use crate::domain::syntax::SyntaxCheckResult;
use crate::domain::test::TestRunResult;
use crate::use_cases::build_project;
use crate::use_cases::check_syntax;
use crate::use_cases::context::ExecutionContext;
use crate::use_cases::dump_config;
use crate::use_cases::launch_app;
use crate::use_cases::request::{
    BuildRequest, DumpRequest, LaunchRequest, SyntaxRequest, TestRequest,
};
use crate::use_cases::result::UseCaseResult;
use crate::use_cases::run_tests;

/// Thin indirection layer used by the MCP service to call use cases.
pub trait McpUseCasePort {
    fn build_project(
        &self,
        context: &ExecutionContext,
        config: &AppConfig,
        request: &BuildRequest,
    ) -> UseCaseResult<BuildResult>;

    fn run_tests(
        &self,
        context: &ExecutionContext,
        config: &AppConfig,
        request: &TestRequest,
    ) -> UseCaseResult<TestRunResult>;

    fn dump_config(
        &self,
        context: &ExecutionContext,
        config: &AppConfig,
        request: &DumpRequest,
    ) -> UseCaseResult<DumpResult>;

    fn launch_app(
        &self,
        context: &ExecutionContext,
        config: &AppConfig,
        request: &LaunchRequest,
    ) -> UseCaseResult<LaunchResult>;

    fn check_syntax(
        &self,
        context: &ExecutionContext,
        config: &AppConfig,
        request: &SyntaxRequest,
    ) -> UseCaseResult<SyntaxCheckResult>;
}

/// Production port implementation delegating directly to use cases.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultMcpUseCasePort;

impl McpUseCasePort for DefaultMcpUseCasePort {
    fn build_project(
        &self,
        context: &ExecutionContext,
        config: &AppConfig,
        request: &BuildRequest,
    ) -> UseCaseResult<BuildResult> {
        build_project::execute(context, config, request)
    }

    fn run_tests(
        &self,
        context: &ExecutionContext,
        config: &AppConfig,
        request: &TestRequest,
    ) -> UseCaseResult<TestRunResult> {
        run_tests::execute(context, config, request)
    }

    fn dump_config(
        &self,
        context: &ExecutionContext,
        config: &AppConfig,
        request: &DumpRequest,
    ) -> UseCaseResult<DumpResult> {
        dump_config::execute(context, config, request)
    }

    fn launch_app(
        &self,
        context: &ExecutionContext,
        config: &AppConfig,
        request: &LaunchRequest,
    ) -> UseCaseResult<LaunchResult> {
        launch_app::execute(context, config, request)
    }

    fn check_syntax(
        &self,
        context: &ExecutionContext,
        config: &AppConfig,
        request: &SyntaxRequest,
    ) -> UseCaseResult<SyntaxCheckResult> {
        check_syntax::execute(context, config, request)
    }
}
