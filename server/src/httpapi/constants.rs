use std::time::Duration;

pub const HEALTH_PATH: &str = "/healthz";

pub const MAX_CHANGE_BODY_BYTES: usize = 16 << 10;
pub const MAX_PLAN_BODY_BYTES: usize = 128 << 10;
pub const MAX_CHANGE_REQUEST_CHARS: usize = 2000;
pub const ROLLBACK_ACCEPTED_MESSAGE: &str =
    "Откат принят. Он вернёт последнюю правку и выложит результат за пару минут.";
pub const CHANGE_ACCEPTED_MESSAGE: &str = "Заявка принята. ИИ подготовит изменение за несколько минут. Когда оно появится в списке, проверьте и примите его.";

pub const MAX_INGEST_BODY_BYTES: usize = 16 << 20;
pub const MAX_INGEST_MESSAGES: usize = 2000;
pub const MAX_CLIENT_ERROR_BYTES: usize = 32 << 10;
pub const MSG_BAD_INGEST: &str = "Неверный пакет сообщений";
pub const MSG_HISTORY_FAILED: &str = "Не удалось прочитать из базы";
pub const QUERY_CONVERSATION: &str = "conversation";
pub const QUERY_SOURCE: &str = "source";
pub const QUERY_LIMIT: &str = "limit";
pub const QUERY_KIND: &str = "kind";
pub const FIELD_CONVERSATION: &str = "conversation";
pub const MAX_CLIENT_ERROR_FIELD_CHARS: usize = 2000;

pub const FIELD_TEXT: &str = "text";
pub const FIELD_HISTORY: &str = "history";
pub const FIELD_FILES: &str = "files";

pub const MAX_BODY_BYTES: usize = 105 << 20;
pub const MAX_FILES: usize = 8;
pub const AUTH_FAIL_DELAY: Duration = Duration::from_millis(400);
pub const BEARER_PREFIX: &str = "Bearer ";

pub const CONTEXT_ATTACHMENT_TURNS: usize = 3;
pub const HISTORY_ATTACHMENT_MAX_BYTES: i32 = 5 << 20;
pub const HISTORY_SOURCE_SERVER: &str = "server";
pub const HISTORY_SOURCE_CLIENT: &str = "client";

pub const MSG_UNAUTHORIZED: &str = "Неверный пароль";
pub const MSG_BAD_REQUEST: &str = "Не удалось разобрать запрос";
pub const MSG_BAD_HISTORY: &str = "Не удалось разобрать историю беседы";
pub const MSG_EMPTY_MESSAGE: &str = "Напишите текст или приложите файл";
pub const MSG_TOO_MANY_FILES: &str = "Слишком много файлов";
pub const MSG_TOO_LARGE: &str = "Файлы слишком большие";
pub const MSG_UNSUPPORTED_FILE: &str = "Этот тип файла не поддерживается";
pub const MSG_UPSTREAM_FAILED: &str = "Советчик временно недоступен, попробуйте ещё раз";
pub const MSG_CHANGES_DISABLED: &str = "Правка приложения не настроена на сервере";
pub const MSG_CHANGE_TOO_LONG: &str = "Описание изменения слишком длинное";
pub const MSG_BAD_CHANGE_ID: &str = "Неверный номер заявки";
pub const MSG_STATUS_FAILED: &str = "Не удалось узнать статус заявки";
pub const MSG_PLAN_FAILED: &str = "Не удалось обдумать правку, попробуйте ещё раз";
pub const MSG_ROLLBACK_FAILED: &str = "Не удалось запустить откат, попробуйте ещё раз";
pub const MSG_CHANGE_FAILED: &str = "Не удалось отправить заявку на изменение, попробуйте ещё раз";
