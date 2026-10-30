# Советчик

Личный голосовой советчик: большая кнопка записи, текстовый чат, вложения файлов и фото, озвучка ответов. Фронт: Angular 22 PWA (`web/`). Сервер с ИИ z.ai (GLM): Go (`server/`).

## Сервер (из `server/`)

```bash
go run .           # слушает :8080, читает server/.env
go test ./...      # юнит-тесты
```

Переменные окружения (локально можно в `server/.env`, на хостинге в его настройках):

| Переменная | Назначение |
|---|---|
| `ZAI_API_KEY` | ключ z.ai (кодинг-план или обычный API-ключ), обязателен |
| `ACCESS_PASSWORD` | пароль доступа, не короче 8 символов, обязателен |
| `PORT` | порт, по умолчанию 8080 (хостинг задаёт сам) |
| `ALLOWED_ORIGINS` | адреса фронта через запятую для CORS, по умолчанию Pages и localhost:4200 |
| `ADVISOR_MODEL` | модель, по умолчанию `glm-5.3` (также есть `glm-5.3-flash`) |
| `ZAI_BASE_URL` | Anthropic-совместимый эндпоинт z.ai, по умолчанию `https://api.z.ai/api/anthropic` |
| `GITHUB_TOKEN` | токен для запуска правок приложения из чата, необязателен (см. раздел «Правка приложения из чата») |
| `DATABASE_URL` | строка подключения Postgres (Neon) для истории и журнала ошибок, необязательна: без неё сервер работает, но ничего не сохраняет (см. раздел «История и журнал ошибок») |

API: `POST /api/chat` (multipart: `text`, `history` как JSON `[{role,text}]`, `files`), заголовок `Authorization: Bearer <пароль>`, ответ `{"reply": "..."}`. `GET /healthz` для проверки хостингом. Фронт запоминает пароль в `localStorage` и просит ввести его заново при ответе 401.

Адрес боевого сервера задаётся в `web/src/app/constants/api.constants.ts` (`API_PRODUCTION_URL`), в dev-режиме фронт ходит на `http://localhost:8080`.

## История и журнал ошибок (Postgres)

Сервер пишет в Postgres, если задана `DATABASE_URL` (бесплатная база на Neon, строка вида `postgres://user:pass@host/db?sslmode=require`). Таблицы создаются сами при старте из `server/internal/store/migrations/*.sql` (применённые миграции помнит таблица `schema_migrations`). Запись идёт в фоне: если база недоступна, чат продолжает работать, а в логе сервера появляется предупреждение.

| Таблица | Что хранит |
|---|---|
| `messages` | все реплики: чат приложения (`source=app`, беседа = `conversation`, меняется кнопкой «Очистить»), сессии Claude Code (`source=claude-code`) и Zcode (`source=zcode`) |
| `events` | запросы чата, планы правок, заявки на правку и откат, пакеты загрузки |
| `error_log` | ошибки сервера (всё, что пишется в лог на уровне ERROR, паники) и ошибки приложения в браузере (`source=client`) |

Чтение (заголовок `Authorization: Bearer <пароль>`): `GET /api/history?source=&conversation=&limit=`, `GET /api/events?kind=&limit=`, `GET /api/errors?limit=`. Запись: `POST /api/ingest` (`{source, conversationId, messages:[{externalId, role, text, at}]}`, повтор с тем же `externalId` не создаёт дубль) и `POST /api/client-error`.

Переписку Claude Code (включая этот проект) загружает скрипт `scripts/ingest-claude-sessions.mjs`:

```bash
ADVISOR_URL=https://interceptor.tail969ed0.ts.net:8443 ADVISOR_PASSWORD=... node scripts/ingest-claude-sessions.mjs
```

Без аргументов он берёт все сессии текущего проекта из `~/.claude/projects/`, можно указать папку или один `.jsonl`. С флагом `--hook` читает с stdin JSON хука Stop (`transcript_path`) и грузит только текущую сессию. В базу попадает текст реплик как есть, поэтому токены и пароли, названные в беседе, тоже сохранятся.

Сессии Zcode (теперь в нём идёт основная работа) загружает `scripts/ingest-zcode-sessions.mjs` — он читает базу `~/.zcode/cli/db/db.sqlite` (нужен Node 22.5+):

```bash
ADVISOR_URL=https://interceptor.tail969ed0.ts.net:8443 ADVISOR_PASSWORD=... node scripts/ingest-zcode-sessions.mjs
```

Без аргументов берёт сессии текущей папки; можно указать папку проекта, `--all` (все проекты), `--db <путь>` (другая база) или `--dry-run` (показать, что уйдёт, без отправки). Повторный запуск не создаёт дублей.

## Правка приложения из чата

Кнопка «Править» в шапке включает режим на одно сообщение (можно голосом): текст уходит на `POST /api/change`, сервер запускает workflow `.github/workflows/claude-change.yml` через GitHub API. Workflow ставит OpenCode (открытый агент для кодинга) и работает через z.ai: эндпоинт `https://api.z.ai/api/anthropic`, модель `glm-5.3`, ключ из секрета `ZAI_API_KEY`, конфиг в `opencode.json`. ИИ правит только `web/src` и `web/public`, затем независимо прогоняются тесты и сборка, а ветка `claude/change-<id>` и pull request создаются самим workflow (если тесты или сборка упали, PR будет черновиком). Статус заявки виден прямо в чате и обновляется сам (приложение опрашивает `GET /api/change/{id}`): ждёт очереди, ИИ работает, готово (ссылка на pull request), принято, закрыто, не получилось (ссылка на журнал запуска), ничего не изменено. Заявку и её запуск связывает номер `id`, который сервер генерирует и передаёт в workflow (имя запуска `Claude change <id>`, ветка `claude/change-<id>`); незавершённые заявки продолжают отслеживаться после перезапуска приложения. Изменение попадает на сайт только после того, как вы нажмёте Merge: пуш в `master` запускает обычную публикацию. Файлы в режиме правки не принимаются, сервер без `GITHUB_TOKEN` отвечает 501.

Настройка один раз:

1. Секрет репозитория `ZAI_API_KEY` (ключ z.ai из `server/.env`): `gh secret set -f server/.env` (загрузит все ключи из файла, лишние секреты не мешают). Старый секрет `ANTHROPIC_API_KEY` можно удалить: `gh secret delete ANTHROPIC_API_KEY`.
2. Разрешить Actions создавать pull request: `gh api -X PUT repos/StarcevAlexander/advisor/actions/permissions/workflow -f default_workflow_permissions=read -F can_approve_pull_request_reviews=true`.
3. На GitHub создать fine-grained токен только для репозитория `advisor` с правом Actions: Read and write и записать его в переменную `GITHUB_TOKEN` сервера на Render. Необязательные переменные: `GITHUB_REPO` (по умолчанию `StarcevAlexander/advisor`), `GITHUB_BRANCH` (по умолчанию `master`).

ИИ правит не только `web/src` и `web/public`, но и сервер на Go (`server/`) и базу. Предохранители: в `opencode.json` агенту запрещены правки `.env`, сетевые утилиты и команды git/gh/rm; в `claude-change.yml` после работы агента диф проверяется по списку разрешённых путей — нельзя трогать `go.mod`, `go.sum`, `.env`, `.github/`, `opencode.json`, `render.yaml`, `scripts/` и зависимости; базу меняют только новой миграцией `NNN_имя.sql` (старые миграции править и удалять нельзя, слова `DROP`, `TRUNCATE`, `DELETE`, `GRANT`, `REVOKE` в миграции запрещены); для правок сервера дополнительно прогоняются `go vet`, `go test` и `go build`. Правка, затронувшая `server/`, сама не сливается: pull request остаётся открытым, и вы принимаете его вручную (переменная `AUTO_MERGE_SERVER` в начале `claude-change.yml`, `"true"` включает автослияние). Откат правки с миграцией не выполняется: git не вернёт базу, такой откат делается вручную.

Откат правки: в настройках (шестерёнка) пункт «Откатить последнюю правку» с окном подтверждения. Он вызывает `POST /api/rollback`, сервер запускает workflow `.github/workflows/claude-rollback.yml`. Workflow берёт последний принятый pull request из ветки `claude/change-*`, который ещё не откатывали (откаты помечены в описании строкой `Откат заявки №N`), делает `git revert`, прогоняет тесты и сборку, сливает откат (ветка `claude/rollback-<id>`, имя запуска `Claude rollback <id>`) и запускает публикацию. Ход отката виден в чате (`GET /api/rollback/{id}`). Если после той правки менялись те же места, откат не применится чисто: запуск завершится ошибкой и ничего не изменит. Для проверки без настоящего GitHub у сервера есть необязательная переменная `GITHUB_API_URL` (по умолчанию `https://api.github.com`).

После Merge версия в шапке не растёт (хук версии работает только на локальных коммитах), но обновление приложения подтягивается, потому что service worker сравнивает содержимое файлов.

## Команды (из `web/`)

```bash
npm start          # dev-сервер http://localhost:4200 (service worker выключен)
npm run build      # боевая сборка в dist/web/browser (service worker активен)
npm test           # юнит-тесты
```

Проверить PWA локально:

```bash
npm run build
npx http-server dist/web/browser -p 4300 -c-1
```

## Версионирование

Версия повышается при каждом коммите хуком `.githooks/pre-commit` (`web/scripts/bump-version.mjs`): правит `web/package.json` и `web/src/app/version.ts`. Хук подключается сам при `npm install` (скрипт `prepare`), либо вручную: `git config core.hooksPath .githooks`.

## Обновления PWA

При каждой загрузке и при возврате на вкладку приложение проверяет новую версию (`AppUpdate`). Найденная версия применяется перезагрузкой, когда пользователь не пишет сообщение, не ждёт ответ и не записывает голос.

## Публикация на GitHub Pages

Workflow `.github/workflows/deploy.yml` собирает `web/` с `--base-href=/<имя-репозитория>/` и публикует на Pages при пуше в `master` или `main`. Pages на бесплатном плане требуют публичный репозиторий. Один раз включить Pages через workflow:

```bash
gh api -X POST repos/StarcevAlexander/advisor/pages -f build_type=workflow
```

Ключ z.ai лежит только в `server/.env` (в `.gitignore`) и в переменных окружения хостинга сервера (на Render — `ZAI_API_KEY`), во фронт и в репозиторий он не попадает.
