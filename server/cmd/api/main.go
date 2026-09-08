// Kairos Go 后端入口：加载配置、连接数据库、执行迁移、启动 chi HTTP 服务。
package main

import (
	"context"
	"errors"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"

	"kairos/server/internal/config"
	"kairos/server/internal/httpapi"
	"kairos/server/internal/store/migrate"
)

func main() {
	log := slog.New(slog.NewTextHandler(os.Stderr, nil))
	slog.SetDefault(log)

	if err := config.LoadEnv(".env", "../.env"); err != nil {
		log.Error("load .env", "error", err)
		os.Exit(1)
	}

	cfg, err := config.Load()
	if err != nil {
		log.Error("load config", "error", err)
		os.Exit(1)
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	pool, err := pgxpool.New(ctx, cfg.DBURL)
	if err != nil {
		log.Error("connect database", "error", err)
		os.Exit(1)
	}
	defer pool.Close()

	if err := migrate.Up(ctx, pool); err != nil {
		log.Error("run migrations", "error", err)
		os.Exit(1)
	}
	log.Info("migrations applied")

	handler := httpapi.New(httpapi.Options{
		Log:          log,
		Username:     cfg.Username,
		PasswordHash: cfg.PasswordHash,
		JWTSecret:    []byte(cfg.JWTSecret),
		JWTTTL:       cfg.JWTTTL,
		DB:           pool,
	})

	srv := &http.Server{
		Addr:              ":" + cfg.Port,
		Handler:           handler,
		ReadHeaderTimeout: 5 * time.Second,
	}

	go func() {
		log.Info("kairos-api listening", "addr", srv.Addr)
		if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			log.Error("server stopped", "error", err)
			stop()
		}
	}()

	<-ctx.Done()
	log.Info("shutting down")
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := srv.Shutdown(shutdownCtx); err != nil {
		log.Error("graceful shutdown", "error", err)
	}
}
