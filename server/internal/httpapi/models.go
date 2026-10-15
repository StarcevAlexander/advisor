package httpapi

import (
	"log/slog"

	"advisor/server/internal/chat"
)

type Options struct {
	Responder      chat.Responder
	Password       string
	AllowedOrigins []string
	Logger         *slog.Logger
}

type ReplyResponse struct {
	Reply string `json:"reply"`
}

type ErrorResponse struct {
	Error string `json:"error"`
}
