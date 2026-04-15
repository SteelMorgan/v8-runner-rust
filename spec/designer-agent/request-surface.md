# Request Surface

Матрица подтвержденной поверхности запроса для `1cv8 DESIGNER /AgentMode` на `8.3.27.1859`.

## Матрица

| Surface | Пример | Prerequisite | Observed output channel | Observed artifact channel | Status |
| --- | --- | --- | --- | --- | --- |
| Startup с явным host key | `1cv8 DESIGNER /F <ib> /AgentMode /AgentListenAddress 127.0.0.1 /AgentPort 1543 /AgentBaseDir <dir> /AgentSSHHostKey <key>` | scratch file ИБ | процесс держит SSH порт `127.0.0.1:1543` | custom `AgentBaseDir` на старте пуст | confirmed-runtime |
| Startup с auto host key | `1cv8 DESIGNER /F <ib> /AgentMode /AgentPort 1544 /AgentSSHHostKeyAuto` | scratch file ИБ | процесс держит SSH порт `127.0.0.1:1544` | серверный host key совпадает с `~/.1cv8/1C/1cv8/host_id` | confirmed-runtime |
| SSH handshake без явного username | `ssh -p 1544 127.0.0.1` | агент запущен | OpenSSH отправляет локального пользователя `alko`; сервер отвечает `password` auth | нет | confirmed-runtime |
| SSH handshake с `PreferredAuthentications=none` | `ssh -o PreferredAuthentications=none -p 1544 127.0.0.1 true` | агент запущен | `Permission denied (password)` | нет | confirmed-runtime |
| SSH password auth с пустым паролем | `ssh -o PreferredAuthentications=password -p 1544 127.0.0.1` + empty password | агент запущен | повторный password prompt, вход не выполняется | нет | confirmed-runtime |
| SFTP вход без явных учетных данных | `sftp -P 1543 127.0.0.1` | агент запущен | password prompt для локального пользователя | нет | confirmed-runtime |
| Remote exec | `ssh -p <port> 127.0.0.1 <remote-command>` | успешная auth | не достигнуто | не достигнуто | blocked-auth |
| Shell session | `ssh -p <port> 127.0.0.1` после успешной auth | успешная auth | не достигнуто | не достигнуто | blocked-auth |
| Namespace/subcommand request | `config ...`, `infobase-tools ...` | успешная auth | не достигнуто | не достигнуто | blocked-auth |
| JSON request/response file mode | неизвестно | успешная auth или встроенный help | не достигнуто | не достигнуто | unconfirmed |

## Ключевые наблюдения

- SSH banner сервера: `libssh_0.9.6`.
- Для `/AgentSSHHostKey` сервер публикует ED25519 host key из явно переданного файла.
- Для `/AgentSSHHostKeyAuto` сервер публикует RSA host key из `~/.1cv8/1C/1cv8/host_id`.
- В проверенном стенде агент не предоставил anonymous auth и не принял пустой пароль.

## Вывод для spec

- Подтверждена только транспортная часть startup и SSH/SFTP handshake.
- Формат прикладного request surface не поднят в separate cards, потому что на этом стенде он полностью заблокирован на этапе SSH userauth.
