package store

import (
	"context"
	"encoding/json"
	"time"
)

type Message struct {
	ID             int64           `json:"id"`
	Source         string          `json:"source"`
	ConversationID string          `json:"conversationId"`
	ExternalID     string          `json:"externalId,omitempty"`
	Role           string          `json:"role"`
	Text           string          `json:"text"`
	Attachments    []string        `json:"attachments"`
	Meta           json.RawMessage `json:"meta,omitempty"`
	CreatedAt      time.Time       `json:"createdAt"`
}

type Event struct {
	ID        int64           `json:"id"`
	Kind      string          `json:"kind"`
	Ref       string          `json:"ref"`
	Payload   json.RawMessage `json:"payload"`
	CreatedAt time.Time       `json:"createdAt"`
}

type ErrorEntry struct {
	ID        int64           `json:"id"`
	Source    string          `json:"source"`
	Message   string          `json:"message"`
	Details   json.RawMessage `json:"details,omitempty"`
	CreatedAt time.Time       `json:"createdAt"`
}

type TurnMeta struct {
	Turn string `json:"turn,omitempty"`
}

type StoredFile struct {
	TurnID string
	Name   string
	Data   []byte
}

type MessageFilter struct {
	Source         string
	ConversationID string
	Limit          int
}

type Recorder interface {
	SaveMessages(messages []Message)
	SaveEvent(kind, ref string, payload any)
	SaveError(source, message string, details any)
	SaveAttachments(conversationID, turnID string, files []StoredFile)
}

type Reader interface {
	Messages(ctx context.Context, filter MessageFilter) ([]Message, error)
	Events(ctx context.Context, kind string, limit int) ([]Event, error)
	Errors(ctx context.Context, limit int) ([]ErrorEntry, error)
	Attachments(ctx context.Context, conversationID string, turnIDs []string, maxBytes int) ([]StoredFile, error)
}

type Store interface {
	Recorder
	Reader
	Close()
}
