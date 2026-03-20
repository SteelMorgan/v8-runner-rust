/// Per-invocation execution metadata shared across transports.
pub mod context;
/// Transport-neutral request DTOs consumed by use cases.
pub mod request;
/// Transport-neutral use-case error and failure contracts.
pub mod result;
/// Build orchestration use case.
pub mod build_project;
/// Syntax-check orchestration use case.
pub mod check_syntax;
/// Dump orchestration use case.
pub mod dump_config;
/// Launch orchestration use case.
pub mod launch_app;
/// Test orchestration use case.
pub mod run_tests;
