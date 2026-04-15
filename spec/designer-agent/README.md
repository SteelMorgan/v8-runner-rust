# Designer Agent Specs

Краткий runtime-ориентированный справочник по `1cv8 DESIGNER /AgentMode` для платформы `8.3.27.1859`.

## Базовый контракт

- Точка входа: `1cv8 DESIGNER /F <file-ib> /AgentMode [...]`.
- Транспорт: SSH и SFTP.
- Один процесс агента обслуживает одну информационную базу.
- Машиночитаемый индекс: `manifest.json`.
- Подробная матрица проверок: `request-surface.md`.

## Что подтверждено

### Startup contract

- Scratch file ИБ создается headless через `CREATEINFOBASE 'File=<dir>;' /DisableStartupDialogs /Out <file>`.
- `DESIGNER /AgentMode` стартует headless на Linux без `DISPLAY`; в лог попадают только предупреждения `colord`, порт при этом открывается.
- Подтверждены параметры `/AgentMode`, `/AgentPort`, `/AgentListenAddress`, `/AgentSSHHostKey`, `/AgentSSHHostKeyAuto`, `/AgentBaseDir`.
- Подтверждены значения по умолчанию:
  - `/AgentPort` -> `1543`.
  - `/AgentListenAddress` -> `127.0.0.1`.
  - `/AgentSSHHostKeyAuto` на Linux -> `~/.1cv8/1C/1cv8/host_id`.
  - `/AgentBaseDir` по локальной справке -> `~/.1cv8/1C/1cv8/<UUID>/sftp`.

### Transport and session contract

- Агент поднимает SSH-сервер с banner `libssh_0.9.6`.
- При запуске с `/AgentSSHHostKey <file>` сервер публикует host key из указанного файла.
- При запуске с `/AgentSSHHostKeyAuto` сервер публикует RSA host key из `/home/alko/.1cv8/1C/1cv8/host_id`.
- Подключение без явного `username` в клиенте не является anonymous mode: OpenSSH автоматически отправляет локального пользователя (`alko` в стенде).
- На этапе `userauth` агент объявляет только метод `password`.
- Пустой пароль отвергается.

## Request Surface

- Подтвержден только транспортный вход в SSH/SFTP и поведение на этапе аутентификации.
- Подтвержденный runtime-факт: без успешного `password`-этапа не удалось дойти до shell-сеанса, `ssh <host> <remote-command>` или SFTP-операций.
- Из-за этого формы запроса команд (`namespace/subcommand`, quoting, каналы stdout/stderr, файловые артефакты ответа) остались частично заблокированы и вынесены в `request-surface.md`.

## Команды агента

Отдельные карточки для команд не созданы: на этом стенде не удалось пройти SSH userauth и выполнить ни одну agent-команду runtime, а локальный help-viewer не выдал отдельных help-страниц по командам агента.

В `README` оставлены только `candidate-doc-only` команды, которые видны в `docs/ru|en/V8Update.htm`:

- `config sign-cfg`
- `config dump-config-to-files`
- `config load-config-from-files`
- `config update-db-cfg`
- `infobase-tools debug-info`
- `infobase-tools restore-ib`

Для `config update-db-cfg` в `V8Update.htm` также видны doc-only опции `--background-finish`, `--session-terminate` и `--session-terminate-message`.

## Success and error surface

- Позитивный startup-кейс: агент открывает TCP-порт и отвечает SSH banner.
- Негативный транспортный кейс: `PreferredAuthentications=none` завершается `Permission denied (password)`.
- Негативный auth-кейс: пустой пароль отклоняется повторным password prompt.
- `DESIGNER --help` без `DISPLAY` в этом окружении не дал usable output и не использовался как источник истины.

## Расхождения Docs/Help/Runtime

| Спорный пункт | Docs/help | Observed runtime | Verdict |
| --- | --- | --- | --- |
| `/AgentSSHHostKeyAuto` на Linux использует `~/.1cv8/1C/1cv8/host_id` | Локальная help-страница `zif3_AgentSSHHostKeyAuto` | Host key fingerprint агента на `:1544` совпал с `host_id` | confirmed-runtime |
| Подключение "без пользователя" | Help этого не обещает; SSH-клиент может не указывать `username` явно | Клиент все равно отправляет локального пользователя `alko` | runtime wins |
| Подключение "без пароля" | Help этого не описывает | Агент предлагает только `password`; пустой пароль отвергается | rejected-for-spec |
| `AgentBaseDir` сразу создает `agentbasedir.json` | В текущем help-viewer это не подтверждено отдельной страницей | На старте без успешной auth/команд никаких файлов в custom `AgentBaseDir` не появилось | unconfirmed |

## Гипотезы и блокеры

- Главный blocker: не подтвержден способ пройти SSH userauth на scratch file ИБ без дополнительных локальных данных о пользователе/пароле агента.
- Из-за этого не подтверждены:
  - формы remote exec;
  - shell-like invocation;
  - каналы stdout/stderr для agent-команд;
  - SFTP artifact flow;
  - success/error JSON payloads;
  - runtime-команды `config/*` и `infobase-tools/*`.
- `candidate-doc-only` факты из `V8Update.htm` оставлены только как кандидаты и не подняты в отдельные карточки.
