package httpapi

import (
	"log/slog"
	"net/http"

	"advisor/server/internal/chat"
)

type Options struct {
	Responder      chat.Responder
	Password       string
	AllowedOrigins []string
	Logger         *slog.Logger
}

type statusRecorder struct {
	http.ResponseWriter
	status int
}

type ReplyResponse struct {
	Reply string `json:"reply"`
}

type ErrorResponse struct {
	Error string `json:"error"`
}
