# ADR-0020: Упростить CLI-only `convert` до repo-aware конвертации текущих исходников проекта

- Статус: `accepted`
- Дата: `2026-04-22`
- Связанные решения: [ADR-0002](0002-izolirovat-runtime-state-po-source-set-pod-workpath.md), [ADR-0005](0005-razdelit-cli-i-mcp-publichnye-poverhnosti.md), [ADR-0006](0006-sohranyat-transportno-neytralnyy-use-case-sloy.md), [ADR-0007](0007-vydelit-otdelnyy-pereklyuchatel-dlya-shared-edt.md), [ADR-0011](0011-eksklyuzivnoe-vladenie-workpath-na-vremya-komandy.md), [ADR-0015](0015-atomarnaya-publikatsiya-dump-artifacts-cherez-staging-backup.md), [ADR-0017](0017-v8project-yaml-source-set-kak-glavnyy-konfiguratsionnyy-kontrakt.md)

## Контекст

`dump` уже является публичным сценарием обратной синхронизации из информационной базы в файлы, но текущая реализация поддерживает только Designer-format target.
Параллельно у пользователей и AI-агентов есть отдельная практическая потребность в простой двусторонней конвертации текущих исходников проекта между EDT и Designer.
Первая редакция этого ADR зафиксировала path-based CLI surface:

1. `convert edt-to-designer --source <path> --target <path>`;
2. `convert designer-to-edt --source <path> --target <path> [--version ...] [--base-project-name ...] [--build]`.

Этот путь решил краткосрочную задачу, но создал новый архитектурный перекос относительно уже принятого config contract из `ADR-0017`:

1. пользователь вынужден указывать `source`, хотя проект уже описывает текущие исходники через `v8project.yaml` и `source-set`;
2. пользователь вынужден выбирать `edt-to-designer` или `designer-to-edt`, хотя направление уже выводится из `config.format`;
3. в public surface просачиваются low-level EDT-параметры `--version`, `--base-project-name` и `--build`, которые не должны быть обязательным знанием обычного пользователя или AI-агента;
4. arbitrary `target` делает `convert` потенциально destructive и плохо согласуется с owned-runtime model под `workPath`;
5. команда получается удобной скорее как thin wrapper над EDT CLI, чем как продуктовая repo-aware операция над текущими исходниками.

При этом смешивать `convert` и `dump` по-прежнему нельзя:

1. `dump` остаётся orchestration-командой "ИБ -> файлы" с builder/source-set semantics;
2. `convert` остаётся файловой конвертацией между форматами текущего проекта;
3. наличие CLI-only `convert` не должно использоваться как аргумент для неявного расширения MCP surface;
4. будущий `dump format=EDT` остаётся отдельным follow-up сценарием, а не alias поверх `convert`.

Нужно одновременно:

1. упростить `convert` до команды, работающей от текущего `v8project.yaml`, а не от произвольных путей;
2. сохранить `convert` CLI-only и не смешивать его с `dump`;
3. убрать из public surface низкоуровневые EDT-флаги и path-based direction toggles;
4. перевести публикацию результата в owned runtime area под `workPath`, чтобы команда по определению не могла переписать исходные каталоги проекта.

## Решение

Считать целевым контрактом отдельную CLI-only команду `convert`, которая конвертирует текущие исходники проекта между EDT и Designer на уровне `source-set`, а не по произвольным путям.

Правила:

1. `convert` является публичной CLI-командой, но не публикуется как MCP tool по умолчанию.
2. Команда не зависит от `builder` и не использует `infobase.connection`; её backend — EDT CLI.
3. Публичный CLI-синтаксис команды — `v8-runner convert [--source-set <name>]`.
4. Без `--source-set` команда обрабатывает все `source-set` из текущего `AppConfig` в их конфигурационном порядке.
5. `--source-set <name>` ограничивает команду одним конкретным `source-set` из текущего проекта; неизвестное имя должно отклоняться на validation boundary.
6. Направление конвертации определяется из `config.format`:
   - `format=EDT` означает `EDT -> Designer`;
   - `format=DESIGNER` означает `Designer -> EDT`.
7. Подкоманды `edt-to-designer` и `designer-to-edt` не являются частью целевого public contract.
8. Флаги `--source`, `--target`, `--version`, `--base-project-name` и `--build` не являются частью целевого public contract.
9. Внутренние EDT import/export параметры должны выводиться из `config.format`, `source-set` semantics, project metadata и tool discovery/config hints, а не требовать явного user-facing флага.
10. Результат `convert` публикуется только в owned generated directories под `workPath/convert/out/<sourceSetName>/<target-format>/`; arbitrary publish target вне `workPath` не поддерживается.
11. Реальная EDT execution должна использовать тот же supported execution model, что и остальные EDT-сценарии: one-shot или shared interactive в зависимости от `tools.edt_cli.interactive_mode`.
12. Runtime state EDT для команды должен жить в отдельном рабочем каталоге `workPath/convert/edt-workspace`, а не переиспользовать `workPath/edt-workspace` из `init` и других EDT-сценариев.
13. Как и другие public команды с runtime state под `workPath`, `convert` должен брать workspace lock на adapter boundary по ADR-0011.
14. Validation, не требующая владения `workPath`, может выполняться до захвата lock, чтобы пользователь получал deterministic validation error раньше workspace-conflict error.
15. Публикация результата должна использовать full-replacement staging/backup contract по ADR-0015, но только внутри owned generated targets команды `convert`; `basePath` и каталоги исходников проекта не являются допустимыми publish target.
16. Текущая path-based реализация `convert` считается transition state и implementation gap относительно этого ADR.
17. Это решение не расширяет контракт `dump`: до отдельной реализации `dump` остаётся поддержан только для `format=DESIGNER`.
18. Потребность в `dump format=EDT` с обратной конвертацией из ИБ в EDT sources официально признаётся целевым follow-up gap и должна быть отражена в backlog и архитектурной документации.
19. Когда `dump format=EDT` будет реализован, он должен остаться отдельной командной семантикой "ИБ -> файлы", а не thin alias поверх `convert`.

## Неграницы (Non-goals)

1. Автоматическая публикация `convert` в MCP.
2. Поддержка произвольных внешних `source`/`target` путей как части целевого public CLI.
3. Частичная или инкрементальная path-based конвертация вне модели `source-set`.
4. Замена `dump` или `build` этой командой.
5. Поддержка прямой Designer/IBCMD-конвертации без EDT CLI.
6. Обещание, что `dump format=EDT` уже реализован.
7. Сохранение low-level EDT flags в user-facing surface только ради обратной совместимости path-based prototype.

## Последствия

1. Пользователь получает более короткий и repo-aware CLI-сценарий без path/direction/platform-specific флагов.
2. Документация должна различать:
   - `dump` как reverse sync из ИБ;
   - `convert` как repo-aware файловую конвертацию текущих исходников проекта между форматами.
3. CLI surface становится шире MCP surface осознанно и явно, что соответствует ADR-0005.
4. Новая команда должна использовать те же архитектурные guardrails:
   - transport-neutral use case boundary;
   - workspace lock;
   - EDT execution mode contract;
   - staging/backup publication внутри owned generated targets.
5. Arbitrary publish target исчезает из целевого контракта, поэтому destructive overlap между `source` и `target` должен быть устранён архитектурно, а не только валидацией.
6. До реализации этого ADR текущая path-based команда и её user-facing docs считаются временным migration state и не должны трактоваться как долгосрочный public contract.
7. До отдельной реализации `dump format=EDT` архитектурная документация обязана явно помечать это как текущий gap, а `convert` — как отдельную repo-aware команду, не подменяющую `dump`.

## План реализации

1. Переписать CLI contract:
   - заменить `convert <direction> --source --target ...` на `convert [--source-set <name>]`;
   - удалить из `clap`-surface `edt-to-designer`, `designer-to-edt`, `--source`, `--target`, `--version`, `--base-project-name`, `--build`.
2. Перевести transport-neutral request/result contract на repo-aware модель:
   - request должен описывать scope (`all` или один `source-set`) и inferred direction;
   - result должен публиковать scope и deterministic output paths под `workPath/convert/out`.
3. Реализовать deterministic output layout:
   - generated output roots под `workPath/convert/out/<sourceSetName>/<target-format>/`;
   - отдельный runtime workspace под `workPath/convert/edt-workspace`;
   - full-replacement publication только внутри owned convert targets.
4. Вывести внутренние EDT import/export параметры из project contract:
   - direction из `config.format`;
   - `source-set` selection из `AppConfig`;
   - internal platform/version/import hints из config/tool discovery и source-set semantics;
   - не возвращать эти low-level knobs в public CLI.
5. Закрыть safety/execution gaps в новой реализации:
   - исключить возможность публикации поверх исходных каталогов проекта;
   - гарантировать использование отдельного shared interactive workspace для `convert`;
   - сохранить `command = "convert"` в JSON pre-dispatch/validation errors.
6. Добавить regression coverage для:
   - `convert` без аргументов как scope "все source-set";
   - `convert --source-set <name>` для одного source-set;
   - inferred direction из `config.format`;
   - deterministic output paths под `workPath/convert/out`;
   - validation-before-lock и busy workspace conflict;
   - one-shot и shared interactive execution paths;
   - запрет destructive overlap по отношению к `basePath`.
7. После фактической реализации синхронизировать:
   - `README.md`;
   - `docs/CAPABILITIES.md`;
   - `docs/DEEP_DIVE.md`;
   - `ARCHITECTURE.md`;
   - arc42 decisions / risks / building blocks;
   - `docs/decisions/README.md`;
   - backlog с explicit follow-up для `dump format=EDT`.

## Верификация

- [x] ADR явно разделяет `dump` и `convert` как разные публичные сценарии.
- [x] ADR фиксирует `convert` как CLI-only команду без автоматической публикации в MCP.
- [x] ADR переводит `convert` на repo-aware contract поверх текущего `v8project.yaml` и `source-set`.
- [x] ADR убирает из целевого public surface explicit direction subcommands и low-level EDT flags.
- [x] ADR фиксирует отдельный runtime workspace под `workPath/convert/edt-workspace`.
- [x] ADR фиксирует, что output публикуется только в owned generated targets под `workPath/convert/out`.
- [x] ADR признаёт текущую path-based реализацию `convert` implementation gap относительно нового контракта.
- [x] ADR признаёт `dump format=EDT` и обратную EDT-конвертацию как реальный follow-up gap, а не как неявное пожелание.
