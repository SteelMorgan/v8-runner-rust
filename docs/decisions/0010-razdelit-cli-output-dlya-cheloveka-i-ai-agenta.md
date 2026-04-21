# ADR-0010: Единый CLI output для человека и AI-агента

- Статус: `accepted`
- Дата: `2026-04-20`

## Контекст

CLI `v8-runner` будет потребляться двумя разными ролями:

1. Человек-разработчик запускает команды вручную и должен быстро увидеть значимые места: что изменилось, что пропущено, что требует внимания, где ошибка и какой следующий шаг.
2. AI-агент запускает CLI как инструмент и тоже нуждается в кратком, предсказуемом, high-signal выводе без лишнего шума.

В предыдущей формулировке решения возникла двусмысленность: нужно ли вводить отдельную public ось `audience/profile`, или достаточно существующей оси `--output text|json` и явных требований к содержанию output.
Проекту нужен единый формат поведения CLI для обеих ролей, а не две разные presentation-модели.
`json` уже является structured contract для автоматизации и не должен меняться только ради различения "человек vs агент".

## Решение

Зафиксировать единый high-signal CLI output contract для человека и AI-агента.

1. Единственная публичная ось CLI output — `--output text|json`.
2. Отдельный public параметр `--audience`, `--profile` или эквивалентный role switch для CLI output не вводится.
3. Оба формата следуют одной и той же информационной политике:
- clean success остаётся кратким и не печатает подробный успешный журнал без необходимости;
- ошибки, warnings, degraded/skipped behavior, артефакты и diagnostic paths должны быть видимы;
- output должен давать минимальный достаточный сигнал для следующего действия;
- нельзя прятать значимые факты в одном формате и терять их в другом.
4. `--output json` остаётся текущим structured format и не меняет свою роль из-за этого ADR.
5. `--output text` остаётся human-readable format, но его текст не должен становиться единственным источником важной машинно значимой информации.
6. Use case слой не знает о presentation rules; правила рендеринга остаются в CLI/output adapter.
7. Любой новый CLI output должен проверяться на три вопроса:
- помогает ли он следующему действию;
- не добавляет ли шум в clean success path;
- сохраняет ли он ошибки, warnings, degraded behavior, artifacts и diagnostics видимыми в `text` и `json`.

## Неграницы (Non-goals)

1. Немедленное изменение всех текущих CLI сообщений в рамках этого ADR.
2. Замена MCP на CLI для агентской автоматизации.
3. Введение отдельного audience-переключателя.
4. Изменение JSON schema только ради разделения ролей.
5. Скрытие ошибок, предупреждений, degraded behavior или путей к диагностике ради краткости.

## Последствия

1. ADR, backlog и пользовательская документация не должны больше описывать CLI output как две public оси.
2. `--output` остаётся единственным публичным переключателем формата CLI output.
3. Clean success path должен оставаться кратким в обоих форматах.
4. JSON остаётся стабильным structured contract для автоматизации и не меняется только из-за различения ролей.
5. Rendering tests должны проверять единый high-signal contract для `text` и `json`.

## План реализации

1. Зафиксировать этот ADR в `docs/decisions`.
2. Добавить invariant в `docs/architecture/invariants.md`.
3. Синхронизировать `ADR-0010`, `docs/architecture/invariants.md`, arc42 и backlog с единой моделью output policy.
4. При реализации unified output policy обновить:
- `src/output/presenter.rs`
- `src/output/text.rs`
- `src/cli/execute.rs`, если меняется wiring/selection rendering rules
- `docs/CAPABILITIES.md`
- `README.md`
5. `src/output/json.rs` не менять без отдельной необходимости и отдельного решения о JSON contract.
6. Ввести и поддерживать rendering tests для единого output contract:
- clean success не шумит;
- ошибки/warnings/degraded/artifacts/actionable diagnostics видимы;
- `text` и `json` не противоречат друг другу по ключевым фактам.
7. При добавлении новой CLI команды или нового result field обновлять rendering tests before exposing the behavior.

## Верификация

- [x] ADR фиксирует единый CLI output contract для человека и AI-агента.
- [x] ADR фиксирует, что единственная public ось output — `--output text|json`.
- [x] ADR явно отвергает отдельный audience/profile-переключатель для CLI output.
- [x] ADR сохраняет текущую роль JSON как structured contract.
- [x] ADR сохраняет транспортно-нейтральный use case слой согласно ADR-0006.
