package httpapi

import "time"

const (
	ChatRoute   = "POST /api/chat"
	HealthRoute = "GET /healthz"
)

const HealthPath = "/healthz"

const (
	ChangeRoute                   = "POST /api/change"
	ChangeStatusRoute             = "GET /api/change/{id}"
	ChangePlanRoute               = "POST /api/change/plan"
	RollbackRoute                 = "POST /api/rollback"
	RollbackStatusRoute           = "GET /api/rollback/{id}"
	MaxChangeBodyBytes      int64 = 16 << 10
	MaxPlanBodyBytes        int64 = 128 << 10
	MaxChangeRequestRunes         = 2000
	RollbackAcceptedMessage       = "Откат принят. Он вернёт последнюю правку Claude и выложит результат за пару минут."
	ChangeAcceptedMessage         = "Заявка принята. Claude подготовит изменение за несколько минут. Когда оно появится в списке, проверьте и примите его."
)

const (
	IngestRoute          = "POST /api/ingest"
	HistoryRoute         = "GET /api/history"
	EventsRoute          = "GET /api/events"
	ErrorsRoute          = "GET /api/errors"
	ClientErrorRoute     = "POST /api/client-error"
	MaxIngestBodyBytes   = 16 << 20
	MaxIngestMessages    = 2000
	MaxClientErrorBytes  = 32 << 10
	MsgBadIngest         = "Неверный пакет сообщений"
	MsgHistoryFailed     = "Не удалось прочитать из базы"
	QueryConversation    = "conversation"
	QuerySource          = "source"
	QueryLimit           = "limit"
	QueryKind            = "kind"
	FieldConversation    = "conversation"
	maxClientErrorFields = 2000
)

const (
	FieldText    = "text"
	FieldHistory = "history"
	FieldFiles   = "files"
)

const (
	MaxBodyBytes  int64 = 105 << 20
	MaxFormMemory int64 = 8 << 20
	MaxFiles            = 8
	AuthFailDelay       = 400 * time.Millisecond
	CORSMaxAge          = "600"
	CORSMethods         = "GET, POST, OPTIONS"
	CORSHeaders         = "Authorization, Content-Type"
	BearerPrefix        = "Bearer "
)

const (
	MsgUnauthorized    = "Неверный пароль"
	MsgBadRequest      = "Не удалось разобрать запрос"
	MsgBadHistory      = "Не удалось разобрать историю беседы"
	MsgEmptyMessage    = "Напишите текст или приложите файл"
	MsgTooManyFiles    = "Слишком много файлов"
	MsgTooLarge        = "Файлы слишком большие"
	MsgUnsupportedFile = "Этот тип файла не поддерживается"
	MsgUpstreamFailed  = "Советчик временно недоступен, попробуйте ещё раз"
	MsgChangesDisabled = "Правка приложения не настроена на сервере"
	MsgChangeTooLong   = "Описание изменения слишком длинное"
	MsgBadChangeID     = "Неверный номер заявки"
	MsgStatusFailed    = "Не удалось узнать статус заявки"
	MsgPlanFailed      = "Не удалось обдумать правку, попробуйте ещё раз"
	MsgRollbackFailed  = "Не удалось запустить откат, попробуйте ещё раз"
	MsgChangeFailed    = "Не удалось отправить заявку на изменение, попробуйте ещё раз"
)
