package httpapi

import (
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"strings"
	"unicode/utf8"

	"advisor/server/internal/chat"
)

type planHandler struct {
	planner chat.Planner
	enabled bool
	logger  *slog.Logger
}

func (h *planHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if h.planner == nil || !h.enabled {
		writeError(w, http.StatusNotImplemented, MsgChangesDisabled)
		return
	}

	r.Body = http.MaxBytesReader(w, r.Body, MaxPlanBodyBytes)
	var body PlanRequest
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

	ctx, cancel := context.WithTimeout(r.Context(), chat.RequestTimeout)
	defer cancel()

	plan, err := h.planner.Plan(ctx, text, body.History)
	if err != nil {
		h.logger.Error("change plan failed", slog.Any("error", err))
		writeError(w, http.StatusBadGateway, MsgPlanFailed)
		return
	}
	h.logger.Info("change planned", slog.Int("steps", len(plan.Steps)))
	writeJSON(w, http.StatusOK, plan)
}
