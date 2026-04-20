# ADR-0007: Выделить отдельный переключатель для shared EDT

- Статус: `accepted`
- Дата: `2026-04-20`

## Контекст

EDT-сценарии могут выполняться через one-shot `1cedtcli`, через обычный interactive EDT CLI mode и через shared EDT execution mechanism.
Shared EDT нужен для долгоживущего исполнения с admission control, очередью, shutdown drain, restart semantics and telemetry.

Обычный EDT CLI mode и shared EDT имеют разные эксплуатационные свойства.
Если управлять ими одним переключателем, пользователи и агенты не смогут ясно выбрать режим исполнения, а изменение MCP shared session behavior будет случайно менять CLI EDT behavior.

## Решение

Shared EDT должен иметь отдельный явный переключатель.

1. Переключатель shared EDT не должен быть тем же самым флагом, что и обычный `tools.edt_cli.interactive_mode`.
2. Обычный EDT CLI должен оставаться доступным и функциональным без включения shared EDT.
3. Shared EDT должен быть доступен как самостоятельный EDT execution mechanism для поддержанных EDT-сценариев.
4. Shared EDT должен стремиться к функциональной полноте обычного EDT CLI для поддержанных операций: `init`, EDT export during `build`, and EDT syntax validation.
5. Если shared EDT временно доступен только для части операций, это должно быть явно описано как gap, а не как архитектурная норма.
6. Shared EDT configuration must describe startup timeout, command timeout, admission behavior, restart behavior and shutdown drain expectations.

## Неграницы (Non-goals)

1. Обязательное удаление one-shot EDT mode.
2. Обязательное удаление обычного `tools.edt_cli.interactive_mode`.
3. Немедленная реализация shared EDT для всех EDT-сценариев в рамках этого ADR.
4. Использование shared EDT как скрытой оптимизации без публичного конфигурационного контракта.

## Последствия

1. Конфигурация должна различать обычный EDT CLI interactive mode и shared EDT mode.
2. Документация должна объяснять, какие EDT-сценарии покрыты shared EDT, а какие остаются gap.
3. MCP shared actor не должен считаться единственным возможным shared EDT API.
4. Реализация shared EDT должна быть проверяема тестами на доступность операций, timeout behavior and shutdown/restart behavior.
5. Future agents must not infer shared EDT enablement from ordinary EDT interactive mode.

## План реализации

1. Зафиксировать этот ADR в `docs/decisions`.
2. Ввести или сохранить отдельный config field for shared EDT, если реализация расширяется за пределы текущего MCP-specific shared actor.
3. Синхронизировать конфигурацию и документацию:
- `src/config/model.rs`
- `src/config/validate.rs`
- `docs/CONFIGURATION.md`
- `docs/CAPABILITIES.md`
- `ARCHITECTURE.md`
4. Если shared EDT покрывает новые сценарии, обновить соответствующие use cases and platform adapters:
- `src/use_cases/init_project.rs`
- `src/use_cases/build_project.rs`
- `src/use_cases/check_syntax.rs`
- `src/platform/edt.rs`
- `src/platform/interactive.rs`
5. Добавить tests for switch independence: ordinary EDT interactive mode and shared EDT mode must be configurable independently.

## Верификация

- [x] ADR требует отдельный переключатель для shared EDT.
- [x] ADR фиксирует, что ordinary EDT CLI должен оставаться доступным и функциональным.
- [x] ADR требует описывать неполное покрытие shared EDT как gap.
- [x] Инвариант добавлен в архитектурную документацию.
