package httpapi

import (
	"context"
	"log/slog"
	"net/http"

	"advisor/server/internal/changes"
)

type rollbackHandler struct {
	requester changes.Requester
	logger    *slog.Logger
}

func (h *rollbackHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if h.requester == nil {
		writeError(w, http.StatusNotImplemented, MsgChangesDisabled)
		return
	}

	ctx, cancel := context.WithTimeout(r.Context(), changes.RequestTimeout)
	defer cancel()

	receipt, err := h.requester.Rollback(ctx)
	if err != nil {
		h.logger.Error("rollback request failed", slog.Any("error", err))
		writeError(w, http.StatusBadGateway, MsgRollbackFailed)
		return
	}
	h.logger.Info("rollback requested", slog.String("id", receipt.ID))
	writeJSON(w, http.StatusAccepted, ChangeResponse{ID: receipt.ID, Message: RollbackAcceptedMessage, URL: receipt.PullsURL})
}

type rollbackStatusHandler struct {
	requester changes.Requester
	logger    *slog.Logger
}

func (h *rollbackStatusHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if h.requester == nil {
		writeError(w, http.StatusNotImplemented, MsgChangesDisabled)
		return
	}

	id := r.PathValue("id")
	if !changes.ValidID(id) {
		writeError(w, http.StatusBadRequest, MsgBadChangeID)
		return
	}

	ctx, cancel := context.WithTimeout(r.Context(), changes.RequestTimeout)
	defer cancel()

	status, err := h.requester.RollbackStatus(ctx, id)
	if err != nil {
		h.logger.Error("rollback status failed", slog.String("id", id), slog.Any("error", err))
		writeError(w, http.StatusBadGateway, MsgStatusFailed)
		return
	}
	writeJSON(w, http.StatusOK, status)
}
