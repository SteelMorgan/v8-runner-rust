# Архитектурные решения (ADR)

Этот каталог хранит архитектурные решения проекта в формате ADR.

## Индекс

- [ADR-0001: Границы поддержки IBCMD как ограниченного backend](0001-granitsy-podderzhki-ibcmd-kak-ogranichennogo-backend.md) — `accepted`, `2026-04-02`
- [ADR-0002: Изолировать runtime state по source-set под workPath](0002-izolirovat-runtime-state-po-source-set-pod-workpath.md) — `accepted`, `2026-04-20`
- [ADR-0003: Поддерживать серверные ИБ для всех инструментов](0003-podderzhivat-servernye-ib-dlya-vseh-instrumentov.md) — `accepted`, `2026-04-20`
- [ADR-0004: Автообнаруживать компоненты платформы 1С по версии-маске](0004-avtoobnaruzhivat-komponenty-platformy-1s-po-versii-maske.md) — `accepted`, `2026-04-20`
- [ADR-0005: Разделить CLI и MCP публичные поверхности](0005-razdelit-cli-i-mcp-publichnye-poverhnosti.md) — `accepted`, `2026-04-20`
- [ADR-0006: Сохранять транспортно-нейтральный use case слой](0006-sohranyat-transportno-neytralnyy-use-case-sloy.md) — `accepted`, `2026-04-20`
- [ADR-0007: Выделить отдельный переключатель для shared EDT](0007-vydelit-otdelnyy-pereklyuchatel-dlya-shared-edt.md) — `accepted`, `2026-04-20`
- [ADR-0008: Держать платформенные backend DSL отдельно от orchestration](0008-derzhat-platformennye-backend-dsl-otdelno-ot-orchestration.md) — `accepted`, `2026-04-20`
- [ADR-0009: Разделить structured business failures и transport/runtime failures](0009-razdelit-business-i-transport-runtime-failures.md) — `accepted`, `2026-04-20`
- [ADR-0010: Разделить CLI output для человека и AI-агента](0010-razdelit-cli-output-dlya-cheloveka-i-ai-agenta.md) — `accepted`, `2026-04-20`

## Правила обновления

- Для изменений архитектурных ограничений добавляйте новый ADR или обновляйте существующий с явным указанием статуса.
- При обновлении публичного контракта синхронизируйте связанные документы (`README.md`, `docs/CAPABILITIES.md`, `docs/DEEP_DIVE.md`, `docs/GIT_WORKFLOW.md`, `ARCHITECTURE.md`).
- Архитектурные инварианты, которые должны соблюдаться агентами и контрибьюторами, перечислены в [docs/architecture/invariants.md](../architecture/invariants.md).
