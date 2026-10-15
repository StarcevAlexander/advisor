package main

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"advisor/server/internal/changes"
	"advisor/server/internal/chat"
	"advisor/server/internal/config"
	"advisor/server/internal/httpapi"
)

const (
	readHeaderTimeout = 10 * time.Second
	readTimeout       = 60 * time.Second
	writeTimeout      = 150 * time.Second
	idleTimeout       = 120 * time.Second
	shutdownTimeout   = 10 * time.Second
)

func main() {
	if err := run(); err != nil {
		slog.Error("server stopped", slog.Any("error", err))
		os.Exit(1)
	}
}

func run() error {
	if err := config.LoadDotEnv(config.DotEnvFileName); err != nil {
		return err
	}
	cfg, err := config.Load()
	if err != nil {
		return fmt.Errorf("config: %w", err)
	}

	var changeRequester changes.Requester
	if cfg.GitHubToken != "" {
		changeRequester = changes.NewGitHub(changes.GitHubConfig{
			Token:  cfg.GitHubToken,
			Repo:   cfg.GitHubRepo,
			Branch: cfg.GitHubBranch,
		})
	}

	claude := chat.NewClaude(cfg.APIKey, cfg.Model)
	handler := httpapi.NewHandler(httpapi.Options{
		Responder:      claude,
		Planner:        claude,
		Changes:        changeRequester,
		Password:       cfg.AccessPassword,
		AllowedOrigins: cfg.AllowedOrigins,
		Logger:         slog.Default(),
	})
	server := &http.Server{
		Addr:              ":" + cfg.Port,
		Handler:           handler,
		ReadHeaderTimeout: readHeaderTimeout,
		ReadTimeout:       readTimeout,
		WriteTimeout:      writeTimeout,
		IdleTimeout:       idleTimeout,
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	serverErr := make(chan error, 1)
	go func() {
		slog.Info("listening", slog.String("addr", server.Addr), slog.String("model", cfg.Model))
		serverErr <- server.ListenAndServe()
	}()

	select {
	case err := <-serverErr:
		if errors.Is(err, http.ErrServerClosed) {
			return nil
		}
		return err
	case <-ctx.Done():
		shutdownCtx, cancel := context.WithTimeout(context.Background(), shutdownTimeout)
		defer cancel()
		return server.Shutdown(shutdownCtx)
	}
}
