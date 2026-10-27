package httpapi

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"log/slog"

	"advisor/server/internal/chat"
	"advisor/server/internal/store"
)

func (h *chatHandler) storedHistory(ctx context.Context, conversationID string) ([]chat.Turn, bool) {
	if conversationID == "" {
		return nil, false
	}
	messages, err := h.reader.Messages(ctx, store.MessageFilter{
		Source:         store.SourceApp,
		ConversationID: conversationID,
		Limit:          chat.MaxHistoryTurns,
	})
	if err != nil {
		h.logger.Warn("stored history unavailable, using client history", slog.Any("error", err))
		return nil, false
	}
	if len(messages) == 0 {
		return nil, false
	}

	turns := make([]chat.Turn, 0, len(messages))
	turnIndex := map[string]int{}
	var withFiles []string
	for _, message := range messages {
		turn := chat.Turn{Role: chat.Role(message.Role), Text: message.Text}
		if message.Role == store.RoleUser && len(message.Attachments) > 0 {
			turn.FileNames = message.Attachments
			if turnID := messageTurnID(message); turnID != "" {
				turnIndex[turnID] = len(turns)
				withFiles = append(withFiles, turnID)
			}
		}
		turns = append(turns, turn)
	}

	if len(withFiles) > ContextAttachmentTurns {
		withFiles = withFiles[len(withFiles)-ContextAttachmentTurns:]
	}
	files, err := h.reader.Attachments(ctx, conversationID, withFiles, HistoryAttachmentMaxBytes)
	if err != nil {
		h.logger.Warn("stored attachments unavailable", slog.Any("error", err))
		return turns, true
	}
	for _, file := range files {
		if index, ok := turnIndex[file.TurnID]; ok {
			turns[index].Attachments = append(turns[index].Attachments, chat.Attachment{Name: file.Name, Data: file.Data})
		}
	}
	return turns, true
}

func messageTurnID(message store.Message) string {
	if len(message.Meta) == 0 {
		return ""
	}
	var meta store.TurnMeta
	if err := json.Unmarshal(message.Meta, &meta); err != nil {
		return ""
	}
	return meta.Turn
}

func newTurnID() string {
	buffer := make([]byte, TurnIDBytes)
	_, _ = rand.Read(buffer)
	return hex.EncodeToString(buffer)
}
