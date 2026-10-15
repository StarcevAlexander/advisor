package httpapi

import "time"

const (
	ChatRoute   = "POST /api/chat"
	HealthRoute = "GET /healthz"
)

const HealthPath = "/healthz"

const (
	ChangeRoute                 = "POST /api/change"
	ChangeStatusRoute           = "GET /api/change/{id}"
	MaxChangeBodyBytes    int64 = 16 << 10
	MaxChangeRequestRunes       = 2000
	ChangeAcceptedMessage       = "Заявка принята. Claude подготовит изменение за несколько минут. Когда оно появится в списке, проверьте и примите его."
)

const (
	FieldText    = "text"
	FieldHistory = "history"
	FieldFiles   = "files"
)

const (
	MaxBodyBytes  int64 = 25 << 20
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
	MsgEmptyMessage    = "Нужен текст или файл"
	MsgTooManyFiles    = "Слишком много файлов"
	MsgTooLarge        = "Файлы слишком большие"
	MsgUnsupportedFile = "Этот тип файла не поддерживается"
	MsgUpstreamFailed  = "Советчик временно недоступен, попробуйте ещё раз"
	MsgChangesDisabled = "Правка приложения не настроена на сервере"
	MsgChangeTooLong   = "Описание изменения слишком длинное"
	MsgBadChangeID     = "Неверный номер заявки"
	MsgStatusFailed    = "Не удалось узнать статус заявки"
	MsgChangeFailed    = "Не удалось отправить заявку на изменение, попробуйте ещё раз"
)
