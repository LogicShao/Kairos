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
	"kairos/server/internal/store"
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
	// Store is optional; when set, the authenticated business routes are enabled.
	Store *store.Queries
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

			if opts.Store != nil {
				registerCoreRoutes(protected, opts)
			}
		})
	})

	return r
}

func registerCoreRoutes(protected chi.Router, opts Options) {
	tasks := &handlers.Tasks{Q: opts.Store, Log: opts.Log}
	protected.Route("/tasks", func(rt chi.Router) {
		rt.Get("/", tasks.List)
		rt.Post("/", tasks.Create)
		rt.Patch("/{id}", tasks.Update)
		rt.Delete("/{id}", tasks.Delete)
		rt.Post("/{id}/complete", tasks.Complete)
		rt.Post("/{id}/uncomplete", tasks.Uncomplete)
	})

	courses := &handlers.Courses{Q: opts.Store, Log: opts.Log}
	protected.Route("/courses", func(rt chi.Router) {
		rt.Get("/", courses.List)
		rt.Post("/", courses.Create)
		rt.Patch("/{id}", courses.Update)
		rt.Delete("/{id}", courses.Delete)
		rt.Post("/import-text", courses.ImportText)
		rt.Post("/reset-semester-dates", courses.ResetSemesterDates)
	})

	exams := &handlers.Exams{Q: opts.Store, Log: opts.Log}
	protected.Route("/exams", func(rt chi.Router) {
		rt.Get("/", exams.List)
		rt.Post("/", exams.Create)
		rt.Patch("/{id}", exams.Update)
		rt.Delete("/{id}", exams.Delete)
		rt.Post("/import-text", exams.ImportText)
	})

	semester := &handlers.Semester{Q: opts.Store, Log: opts.Log}
	protected.Get("/semesters", semester.ListSemesters)
	protected.Route("/term-phases", func(rt chi.Router) {
		rt.Get("/", semester.ListPhases)
		rt.Post("/", semester.CreatePhase)
		rt.Patch("/{id}", semester.UpdatePhase)
		rt.Delete("/{id}", semester.DeletePhase)
		rt.Get("/current-status", semester.CurrentStatus)
	})

	calendar := &handlers.Calendar{Q: opts.Store, Log: opts.Log}
	protected.Route("/calendar", func(rt chi.Router) {
		rt.Get("/week", calendar.Week)
		rt.Get("/day", calendar.Day)
	})

	briefing := &handlers.Briefing{Q: opts.Store, Log: opts.Log}
	protected.Get("/briefing/today", briefing.Today)
}
