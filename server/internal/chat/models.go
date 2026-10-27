package chat

import (
	"context"
	"errors"
)

type Role string

type Turn struct {
	Role        Role         `json:"role"`
	Text        string       `json:"text"`
	FileNames   []string     `json:"-"`
	Attachments []Attachment `json:"-"`
}

type Attachment struct {
	Name string
	Data []byte
}

type Conversation struct {
	History     []Turn
	Text        string
	Attachments []Attachment
}

type Responder interface {
	Reply(ctx context.Context, conversation Conversation) (string, error)
}

type Plan struct {
	Summary string   `json:"summary"`
	Steps   []string `json:"steps"`
}

type Planner interface {
	Plan(ctx context.Context, request string, history []Turn) (Plan, error)
}

var (
	ErrUnsupportedAttachment = errors.New("unsupported attachment")
	ErrEmptyReply            = errors.New("empty reply")
	ErrBadPlan               = errors.New("bad plan")
)
