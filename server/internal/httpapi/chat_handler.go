package httpapi

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"log/slog"
	"mime/multipart"
	"net/http"
	"strings"

	"advisor/server/internal/chat"
)

type chatHandler struct {
	responder chat.Responder
	logger    *slog.Logger
}

func (h *chatHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	conversation, status, message := h.readConversation(w, r)
	if message != "" {
		writeError(w, status, message)
		return
	}

	ctx, cancel := context.WithTimeout(r.Context(), chat.RequestTimeout)
	defer cancel()

	reply, err := h.responder.Reply(ctx, conversation)
	switch {
	case errors.Is(err, chat.ErrUnsupportedAttachment):
		writeError(w, http.StatusUnsupportedMediaType, MsgUnsupportedFile)
	case err != nil:
		h.logger.Error("reply failed", slog.Any("error", err))
		writeError(w, http.StatusBadGateway, MsgUpstreamFailed)
	default:
		writeJSON(w, http.StatusOK, ReplyResponse{Reply: reply})
	}
}

func (h *chatHandler) readConversation(w http.ResponseWriter, r *http.Request) (chat.Conversation, int, string) {
	r.Body = http.MaxBytesReader(w, r.Body, MaxBodyBytes)
	if err := r.ParseMultipartForm(MaxFormMemory); err != nil {
		var tooLarge *http.MaxBytesError
		if errors.As(err, &tooLarge) {
			return chat.Conversation{}, http.StatusRequestEntityTooLarge, MsgTooLarge
		}
		return chat.Conversation{}, http.StatusBadRequest, MsgBadRequest
	}
	defer func() { _ = r.MultipartForm.RemoveAll() }()

	var history []chat.Turn
	if raw := r.FormValue(FieldHistory); raw != "" {
		if err := json.Unmarshal([]byte(raw), &history); err != nil {
			return chat.Conversation{}, http.StatusBadRequest, MsgBadHistory
		}
	}

	headers := r.MultipartForm.File[FieldFiles]
	if len(headers) > MaxFiles {
		return chat.Conversation{}, http.StatusBadRequest, MsgTooManyFiles
	}
	attachments, err := readAttachments(headers)
	if err != nil {
		h.logger.Error("read attachments failed", slog.Any("error", err))
		return chat.Conversation{}, http.StatusBadRequest, MsgBadRequest
	}

	text := strings.TrimSpace(r.FormValue(FieldText))
	if text == "" && len(attachments) == 0 {
		return chat.Conversation{}, http.StatusBadRequest, MsgEmptyMessage
	}
	return chat.Conversation{History: history, Text: text, Attachments: attachments}, 0, ""
}

func readAttachments(headers []*multipart.FileHeader) ([]chat.Attachment, error) {
	attachments := make([]chat.Attachment, 0, len(headers))
	for _, header := range headers {
		data, err := readFile(header)
		if err != nil {
			return nil, err
		}
		attachments = append(attachments, chat.Attachment{Name: header.Filename, Data: data})
	}
	return attachments, nil
}

func readFile(header *multipart.FileHeader) ([]byte, error) {
	file, err := header.Open()
	if err != nil {
		return nil, err
	}
	defer file.Close()
	return io.ReadAll(file)
}
