// Package httpapi assembles the chi router, middleware chain and handlers.
package httpapi

import (
	"context"
	"log/slog"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"

	"kairos/server/internal/httpapi/handlers"
	"kairos/server/internal/httpapi/middleware"
)

// Options configures the router.
type Options struct {
	Log          *slog.Logger
	Username     string
	PasswordHash string
	JWTSecret    []byte
	JWTTTL       time.Duration
	// DB is optional; when set, /healthz pings it.
	DB interface {
		Ping(ctx context.Context) error
	}
}

// New builds the chi router with the full middleware chain and routes.
func New(opts Options) http.Handler {
	r := chi.NewRouter()

	r.Use(middleware.RequestID)
	r.Use(middleware.Logger(opts.Log))
	r.Use(middleware.Recover(opts.Log))

	r.Get("/healthz", handlers.Healthz(opts.DB))
	r.Get("/healthz/live", handlers.Healthz(nil))

	auth := &handlers.Auth{
		Username:     opts.Username,
		PasswordHash: opts.PasswordHash,
		JWTSecret:    opts.JWTSecret,
		TTL:          opts.JWTTTL,
		Log:          opts.Log,
	}

	r.Route("/api", func(api chi.Router) {
		api.Post("/auth/login", auth.Login)
		api.Group(func(protected chi.Router) {
			protected.Use(middleware.Auth(opts.JWTSecret))
			protected.Post("/auth/logout", auth.Logout)
			protected.Get("/auth/me", auth.Me)
		})
	})

	return r
}
