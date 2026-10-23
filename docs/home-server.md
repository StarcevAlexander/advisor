# Сервер советчика на своём ПК

Схема как у post_helper: Go-сервер и база живут на вашем ПК, телефон ходит к ним через Tailscale Funnel. Сайт остаётся на GitHub Pages. Оба проекта делят один ПК, одну установку Postgres и один Tailscale.

| | post_helper | советчик |
|---|---|---|
| Порт сервера | 8080 | 8081 |
| Публичный адрес | `https://interceptor.tail969ed0.ts.net` | `https://interceptor.tail969ed0.ts.net:8443` |
| База | схема `public` базы `posthelper` | схема `advisor` той же базы |
| Задача автозапуска | `PostHelperServer` | `AdvisorServer` |

Цена выбора: если ПК выключен или нет интернета, советчик не работает. Ночную копию базы делает задача post_helper (`PostHelperBackup`), она снимает всю базу `posthelper`, то есть и схему `advisor`.

## Установка

1. Секреты: `powershell -File tools/server/env.ps1`. Пароль доступа сгенерируется и попадёт в буфер обмена, введите его в приложении. `GITHUB_TOKEN` (правки из чата) можно добавить позже строкой в `server\.env`.
2. От администратора: `powershell -Command "Start-Process powershell -Verb RunAs -ArgumentList '-NoExit','-File','C:\Users\master\WebstormProjects\advisor\tools\server\autostart.ps1','install'"` соберёт `server\advisor.exe` и поставит задачу `AdvisorServer` на запуск при включении ПК.
3. Публикация наружу: `powershell -File tools/server/funnel.ps1` (порт 8443 нужен, потому что 443 занят post_helper).
4. Проверка с телефона по мобильному интернету: `https://interceptor.tail969ed0.ts.net:8443/healthz` отвечает пустой страницей.

Состояние: `powershell -File tools/server/autostart.ps1 status`. После правок кода сервера повторите шаг 2.

## Правки из чата

Сервер сам запускает workflow на GitHub, поэтому ему нужен `GITHUB_TOKEN`. Принятый pull request обновляет код в репозитории, но не пересобирает сервер на ПК: после Merge правки `server/` выполните `git pull` и шаг 2.

Render больше не нужен: сервис `advisor-server` можно остановить, когда новый адрес проверен.
