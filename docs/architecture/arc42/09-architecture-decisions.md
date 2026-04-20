## 9. Архитектурные решения

Существующие ADR-файлы:

- [ADR-0001: Границы поддержки IBCMD как ограниченного backend](../../decisions/0001-granitsy-podderzhki-ibcmd-kak-ogranichennogo-backend.md)
- [ADR-0002: Изолировать runtime state по source-set под workPath](../../decisions/0002-izolirovat-runtime-state-po-source-set-pod-workpath.md)
- [ADR-0003: Поддерживать серверные ИБ для всех инструментов](../../decisions/0003-podderzhivat-servernye-ib-dlya-vseh-instrumentov.md)
- [ADR-0004: Автообнаруживать компоненты платформы 1С по версии-маске](../../decisions/0004-avtoobnaruzhivat-komponenty-platformy-1s-po-versii-maske.md)
- [ADR-0005: Разделить CLI и MCP публичные поверхности](../../decisions/0005-razdelit-cli-i-mcp-publichnye-poverhnosti.md)
- [ADR-0006: Сохранять транспортно-нейтральный use case слой](../../decisions/0006-sohranyat-transportno-neytralnyy-use-case-sloy.md)
- [ADR-0007: Выделить отдельный переключатель для shared EDT](../../decisions/0007-vydelit-otdelnyy-pereklyuchatel-dlya-shared-edt.md)
- [ADR-0008: Держать платформенные backend DSL отдельно от orchestration](../../decisions/0008-derzhat-platformennye-backend-dsl-otdelno-ot-orchestration.md)
- [ADR-0009: Разделить structured business failures и transport/runtime failures](../../decisions/0009-razdelit-business-i-transport-runtime-failures.md)
- [ADR-0010: Разделить CLI output для человека и AI-агента](../../decisions/0010-razdelit-cli-output-dlya-cheloveka-i-ai-agenta.md)

Архитектурные инварианты для агентов и контрибьюторов зафиксированы в [docs/architecture/invariants.md](../invariants.md).

Важные уже реализованные решения, которые сейчас зафиксированы кодом и внутренними архитектурными заметками:

- транспортно-нейтральные контракты use case, общие для CLI и MCP, формализованы в ADR-0006;
- отдельные платформенные адаптеры для Designer, Enterprise, IBCMD и EDT формализованы в ADR-0008;
- централизованный поиск компонентов платформы 1С по версии или версии-маске;
- общий интерактивный EDT actor ограничен MCP EDT syntax, а не всеми EDT-операциями;
- CLI и MCP intentionally expose different public surfaces: MCP не зеркалит CLI полностью, см. ADR-0005;
- текущая поддержка `builder=IBCMD` ограничена файловыми ИБ, но целевой контракт требует server infobase support для всех инструментов;
- сохранённое инкрементальное состояние хранится в per-source-set `redb` contexts под `workPath`;
- presentation concerns (`Presenter`, `Envelope`, text formatting) остаются вне use case;
- разделение business failures и transport/runtime failures формализовано в ADR-0009;
- shared EDT должен иметь отдельный переключатель и явно описанные gaps, см. ADR-0007;
- CLI output должен различать human highlights и agent minimal signal, см. ADR-0010.

Рекомендуемое развитие:

- фиксировать эти решения в явных ADR, когда они меняются или когда добавляются новые backend/transport;
- при изменении любого инварианта сначала обновлять соответствующий ADR.
