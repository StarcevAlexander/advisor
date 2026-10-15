package httpapi

import (
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"strings"
	"unicode/utf8"

	"advisor/server/internal/changes"
)

type changeHandler struct {
	requester changes.Requester
	logger    *slog.Logger
}

func (h *changeHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if h.requester == nil {
		writeError(w, http.StatusNotImplemented, MsgChangesDisabled)
		return
	}

	r.Body = http.MaxBytesReader(w, r.Body, MaxChangeBodyBytes)
	var body ChangeRequest
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		writeError(w, http.StatusBadRequest, MsgBadRequest)
		return
	}

	text := strings.TrimSpace(body.Request)
	switch {
	case text == "":
		writeError(w, http.StatusBadRequest, MsgEmptyMessage)
		return
	case utf8.RuneCountInString(text) > MaxChangeRequestRunes:
		writeError(w, http.StatusBadRequest, MsgChangeTooLong)
		return
	}

	ctx, cancel := context.WithTimeout(r.Context(), changes.RequestTimeout)
	defer cancel()

	receipt, err := h.requester.Request(ctx, text)
	if err != nil {
		h.logger.Error("change request failed", slog.Any("error", err))
		writeError(w, http.StatusBadGateway, MsgChangeFailed)
		return
	}
	h.logger.Info("change requested", slog.Int("runes", utf8.RuneCountInString(text)))
	writeJSON(w, http.StatusAccepted, ChangeResponse{Message: ChangeAcceptedMessage, URL: receipt.PullsURL})
}
