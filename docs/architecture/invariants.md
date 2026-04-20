# Архитектурные инварианты

Этот документ фиксирует правила, которые должны оставаться верными при развитии `v8-runner`.
Если изменение нарушает инвариант, сначала нужен новый ADR, который явно заменяет или уточняет текущее решение.

## Цель продукта

1. Главная цель `v8-runner` — предоставить простой и удобный интерфейс для сборки и проверки исходников 1С-решения человеком и AI-агентом.
2. Основной пользовательский цикл — `build -> syntax/test -> diagnose`.
3. Новая функциональность должна упрощать этот цикл или явно объяснять, какую диагностическую, эксплуатационную или интеграционную задачу она закрывает.
4. Низкоуровневые детали утилит 1С не должны становиться обязательным знанием для обычного пользователя или AI-агента, если их можно скрыть за стабильным CLI/MCP контрактом.
5. Удобство для человека и пригодность для AI-агента являются равноправными критериями продукта.

## Публичные поверхности

1. CLI и MCP являются разными публичными поверхностями.
2. MCP не зеркалит CLI автоматически.
3. Текущая MCP-поверхность состоит из 8 tool-операций: `run_all_tests`, `run_module_tests`, `build_project`, `dump_config`, `launch_app`, `check_syntax_edt`, `check_syntax_designer_config`, `check_syntax_designer_modules`.
4. Добавление, удаление или переименование MCP tool-операций является изменением публичного контракта и требует отдельного ADR или явного обновления действующего ADR.

См. [ADR-0005](../decisions/0005-razdelit-cli-i-mcp-publichnye-poverhnosti.md).

## Use Case Layer

1. `src/use_cases` остается транспортно-нейтральным orchestration-слоем.
2. Use case не зависят от `clap`, CLI `Presenter`, CLI `Envelope`, MCP DTO и конкретного transport payload format.
3. CLI и MCP адаптеры преобразуют свои входные DTO/аргументы в `use_cases::request::*`.
4. Presentation, envelope rendering и MCP tool payload formatting остаются за пределами use case.

См. [ADR-0006](../decisions/0006-sohranyat-transportno-neytralnyy-use-case-sloy.md).

## Shared EDT

1. Shared EDT execution mode должен иметь отдельный явный переключатель, не смешанный с обычным one-shot/interactive EDT CLI режимом.
2. Shared EDT должен развиваться как полноценный EDT execution backend для поддержанных EDT-сценариев, а не как скрытая оптимизация только для одного MCP tool.
3. Если shared EDT временно покрывает не все EDT-сценарии, gap должен быть зафиксирован в документации или ADR.

См. [ADR-0007](../decisions/0007-vydelit-otdelnyy-pereklyuchatel-dlya-shared-edt.md).

## Platform Backends

1. Низкоуровневые DSL для платформенных инструментов остаются в `src/platform`.
2. `DesignerDsl`, `IbcmdDsl`, `EdtDsl`, `EnterpriseDsl`, `platform::locator`, `platform::process` и interactive executor не должны протаскивать process details в presentation или transport adapters.
3. Orchestration вызывает backend DSL через доменные операции и анализирует `PlatformCommandResult`, но не собирает сырые process arguments выше платформенного слоя.
4. Новый backend добавляется как отдельный adapter/DSL с явными gap и матрицей поддержки.

См. [ADR-0008](../decisions/0008-derzhat-platformennye-backend-dsl-otdelno-ot-orchestration.md).

## Failures

1. Business failures и transport/runtime failures разделены.
2. Use case возвращают `UseCaseFailure<T>` с transport-neutral metadata и, где возможно, структурированным payload.
3. MCP service разделяет `McpBusinessFailure<T>` и `McpInternalError`.
4. Orchestration не знает, как CLI или MCP сериализуют ошибку наружу.

См. [ADR-0009](../decisions/0009-razdelit-business-i-transport-runtime-failures.md).

## CLI Output

1. CLI output проектируется для двух потребителей: человека и AI-агента.
2. Human-oriented output должен акцентировать значимые места: итог, ошибки, предупреждения, degraded behavior, созданные артефакты, пути к диагностике и следующий actionable hint.
3. Agent-oriented output должен быть кратким: при чистом успехе не выводить лишний пошаговый журнал, при ошибке давать только минимальный actionable signal.
4. Формат вывода (`text`/`json`) и аудитория вывода (`human`/`agent`) являются разными осями; `json` не означает автоматически verbose output.
5. Use case слой не знает audience-specific rendering rules.

См. [ADR-0010](../decisions/0010-razdelit-cli-output-dlya-cheloveka-i-ai-agenta.md).
