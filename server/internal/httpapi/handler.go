package httpapi

import (
	"log/slog"
	"net/http"
)

func NewHandler(options Options) http.Handler {
	logger := options.Logger
	if logger == nil {
		logger = slog.Default()
	}

	mux := http.NewServeMux()
	mux.HandleFunc(HealthRoute, func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusNoContent)
	})
	mux.Handle(ChatRoute, withPassword(options.Password, &chatHandler{
		responder: options.Responder,
		logger:    logger,
	}))
	return withCORS(options.AllowedOrigins, withLogging(logger, mux))
}
