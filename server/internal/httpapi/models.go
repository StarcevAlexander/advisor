package httpapi

import (
	"log/slog"
	"net/http"
	"time"

	"advisor/server/internal/changes"
	"advisor/server/internal/chat"
	"advisor/server/internal/store"
)

type Options struct {
	Responder      chat.Responder
	Planner        chat.Planner
	Changes        changes.Requester
	Password       string
	AllowedOrigins []string
	Store          store.Store
	Logger         *slog.Logger
}

type IngestRequest struct {
	Source         string          `json:"source"`
	ConversationID string          `json:"conversationId"`
	Messages       []IngestMessage `json:"messages"`
}

type IngestMessage struct {
	ExternalID string    `json:"externalId"`
	Role       string    `json:"role"`
	Text       string    `json:"text"`
	At         time.Time `json:"at"`
}

type IngestResponse struct {
	Accepted int `json:"accepted"`
}

type ClientErrorRequest struct {
	Message string `json:"message"`
	Where   string `json:"where"`
	Stack   string `json:"stack"`
	Version string `json:"version"`
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

type PlanRequest struct {
	Request string      `json:"request"`
	History []chat.Turn `json:"history"`
}

type ChangeResponse struct {
	ID      string `json:"id"`
	Message string `json:"message"`
	URL     string `json:"url"`
}

type ErrorResponse struct {
	Error string `json:"error"`
}
