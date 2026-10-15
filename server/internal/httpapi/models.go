package httpapi

import (
	"log/slog"
	"net/http"

	"advisor/server/internal/changes"
	"advisor/server/internal/chat"
)

type Options struct {
	Responder      chat.Responder
	Changes        changes.Requester
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

type ChangeRequest struct {
	Request string `json:"request"`
}

type ChangeResponse struct {
	ID      string `json:"id"`
	Message string `json:"message"`
	URL     string `json:"url"`
}

type ErrorResponse struct {
	Error string `json:"error"`
}
