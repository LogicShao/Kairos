package notify

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"log/slog"
	"sync"
	"time"

	"github.com/jackc/pgx/v5/pgtype"

	"kairos/server/internal/domain/pomodoro"
	"kairos/server/internal/domain/termphase"
	"kairos/server/internal/store"
)

var errEmptyOffsets = errors.New("exam_offsets 为空")

const (
	categoryExams = "exams"
	categoryTasks = "tasks"
	categoryAI    = "ai"

	taskRetryInterval = time.Hour
	oneshotGrace      = time.Minute
)

var defaultExamOffsets = []int{1440, 60}

// BriefGenerator is the subset of ai.Generator the scheduler needs. The
// integration layer passes *ai.Generator; the interface keeps this domain free
// of any dependency on package ai.
type BriefGenerator interface {
	GenerateBrief(ctx context.Context, date string, force bool, onDelta func(string)) (*store.AiMorningBrief, error)
}

// Scheduler owns every server-side notification timer: exam offsets, daily and
// one-shot task reminders, and the 07:00 (+08:00) AI brief. Now is injectable
// for tests; all other state is guarded by mu.
type Scheduler struct {
	q      *store.Queries
	mailer *Mailer
	gen    BriefGenerator
	log    *slog.Logger

	// Now returns the current time; defaults to time.Now.
	Now func() time.Time

	mu      sync.Mutex
	ctx     context.Context
	started bool
	timers  map[string][]*time.Timer
}

// NewScheduler builds a scheduler. A nil logger falls back to slog.Default.
func NewScheduler(q *store.Queries, mailer *Mailer, gen BriefGenerator, log *slog.Logger) *Scheduler {
	if log == nil {
		log = slog.Default()
	}
	return &Scheduler{
		q:      q,
		mailer: mailer,
		gen:    gen,
		log:    log,
		Now:    time.Now,
		timers: make(map[string][]*time.Timer),
	}
}

// Start clears expired one-shot reminders, registers all three timer categories
// and stops every timer when ctx is cancelled.
func (s *Scheduler) Start(ctx context.Context) {
	if ctx == nil {
		ctx = context.Background()
	}

	s.mu.Lock()
	s.ctx = ctx
	firstStart := !s.started
	s.started = true
	s.mu.Unlock()

	if firstStart {
		go func() {
			<-ctx.Done()
			s.stopAll()
		}()
	}

	s.clearExpiredRemindAts(ctx)
	s.RecomputeExams(ctx)
	s.RecomputeTasks(ctx)
	s.RecomputeAI(ctx)
}

// RecomputeExams cancels every exam timer and rebuilds them from the current
// config, term phase and exam list.
func (s *Scheduler) RecomputeExams(ctx context.Context) {
	s.register(categoryExams, s.planExamTimers(ctx))
}

// RecomputeTasks cancels the task timer and re-arms it from the current task
// list. One-shots that are past their grace window are skipped, not fired.
func (s *Scheduler) RecomputeTasks(ctx context.Context) {
	tasks, err := s.q.ListTasks(ctx, store.ListTasksParams{})
	if err != nil {
		s.logger().Warn("读取任务列表失败，跳过任务提醒调度", "error", err)
		return
	}

	now := s.now()
	reminders := remindersFromTasks(tasks)
	baseCtx := s.baseCtx()

	fire, due, ok := nextTaskBatch(reminders, now)
	if !ok {
		s.register(categoryTasks, []scheduled{{
			fire: now.Add(taskRetryInterval),
			fn: s.wrapped(baseCtx, func() {
				s.RecomputeTasks(baseCtx)
			}),
		}})
		return
	}

	due = append([]reminder(nil), due...)
	s.register(categoryTasks, []scheduled{{
		fire: fire,
		fn: s.wrapped(baseCtx, func() {
			s.fireTaskReminders(baseCtx, due)
			s.RecomputeTasks(baseCtx)
		}),
	}})
}

// RecomputeAI cancels the AI timer and re-arms the next 07:00 (+08:00) run when
// AI is enabled and configured, otherwise leaves it cancelled.
func (s *Scheduler) RecomputeAI(ctx context.Context) {
	cfg, err := s.q.GetAiConfig(ctx)
	if err != nil {
		s.logger().Warn("读取 AI 配置失败，跳过晨报调度", "error", err)
		return
	}
	if !cfg.Enabled || cfg.ApiKeyEncrypted == "" {
		s.logger().Debug("AI 未启用或缺少密钥，取消晨报调度")
		s.register(categoryAI, nil)
		return
	}
	s.scheduleNextSevenAM()
}

// PhaseEnded implements pomodoro.Notifier: it sends the phase-finished email
// asynchronously so the finish-phase request never blocks on SMTP.
func (s *Scheduler) PhaseEnded(ev pomodoro.PhaseEndedEvent) {
	if s == nil || s.mailer == nil || !s.mailer.Enabled() {
		return
	}

	subject, body := PomodoroEmail(string(ev.Phase))
	ctx := s.baseCtx()
	go func() {
		defer func() {
			if r := recover(); r != nil {
				s.logger().Error("番茄钟邮件发送 panic", "panic", r)
			}
		}()
		if err := s.mailer.Send(ctx, Message{Subject: subject, Body: body}); err != nil {
			s.logger().Error("番茄钟邮件发送失败", "error", err)
		}
	}()
}

type scheduled struct {
	fire time.Time
	fn   func()
}

func (s *Scheduler) planExamTimers(ctx context.Context) []scheduled {
	cfg, err := s.q.GetNotificationConfig(ctx)
	if err != nil {
		s.logger().Warn("读取通知配置失败，跳过考试提醒调度", "error", err)
		return nil
	}
	if !cfg.Enabled {
		s.logger().Debug("考试通知已禁用，取消考试提醒")
		return nil
	}

	status, err := termphase.GetCurrentPhaseStatus(ctx, s.q, termphase.DefaultSource)
	if err != nil {
		s.logger().Warn("读取学期阶段失败，跳过考试提醒调度", "error", err)
		return nil
	}
	if !status.ExamNotificationsEnabled {
		s.logger().Debug("当前学期阶段禁止考试通知", "phase_type", status.PhaseType)
		return nil
	}

	offsets, err := parseExamOffsets(cfg.ExamOffsets)
	if err != nil {
		s.logger().Warn("exam_offsets 解析失败，回退默认 [1440,60]", "error", err, "raw", string(cfg.ExamOffsets))
		offsets = append([]int(nil), defaultExamOffsets...)
	}

	exams, err := s.q.ListExams(ctx)
	if err != nil {
		s.logger().Warn("读取考试列表失败，跳过考试提醒调度", "error", err)
		return nil
	}

	now := s.now()
	baseCtx := s.baseCtx()
	var items []scheduled
	for _, exam := range exams {
		if !exam.ExamDatetime.Valid {
			s.logger().Warn("跳过考试：exam_datetime 无效", "exam_id", exam.ID, "course", exam.CourseName)
			continue
		}
		examAt := exam.ExamDatetime.Time
		course := exam.CourseName
		for _, offset := range offsets {
			fire := examAt.Add(-time.Duration(offset) * time.Minute)
			if !fire.After(now) {
				continue
			}
			items = append(items, scheduled{
				fire: fire,
				fn: s.wrapped(baseCtx, func() {
					subject, body := ExamEmail(course, offset)
					s.sendSync(baseCtx, Message{Subject: subject, Body: body})
				}),
			})
		}
	}
	return items
}

func (s *Scheduler) scheduleNextSevenAM() {
	now := s.now()
	fire := nextSevenAM(now)
	baseCtx := s.baseCtx()

	s.register(categoryAI, []scheduled{{
		fire: fire,
		fn: s.wrapped(baseCtx, func() {
			s.runMorningBrief(baseCtx)
			s.scheduleNextSevenAM()
		}),
	}})
}

func (s *Scheduler) runMorningBrief(ctx context.Context) {
	if s.gen == nil {
		return
	}
	if err := ctx.Err(); err != nil {
		return
	}
	brief, err := s.gen.GenerateBrief(ctx, "", false, nil)
	if err != nil {
		s.logger().Warn("生成 AI 晨报失败", "error", err)
		return
	}
	if brief == nil {
		return
	}
	s.logger().Debug("AI 晨报已生成，准备发送", "preview", briefPreview(brief.Markdown))
	subject, body := AIBriefEmail(brief.Markdown)
	s.sendSync(ctx, Message{Subject: subject, Body: body})
}

func (s *Scheduler) fireTaskReminders(ctx context.Context, due []reminder) {
	for _, r := range due {
		if err := ctx.Err(); err != nil {
			return
		}
		switch r.Kind {
		case reminderDaily:
			subject, body := DailyTaskEmail(r.Title)
			s.sendSync(ctx, Message{Subject: subject, Body: body})
		case reminderOneShot:
			subject, body := OneShotTaskEmail(r.Title)
			s.sendSync(ctx, Message{Subject: subject, Body: body})
			if err := s.clearRemindAt(ctx, r.ID); err != nil {
				s.logger().Warn("清空一次性提醒失败", "task_id", r.ID, "error", err)
			}
		}
	}
}

func (s *Scheduler) clearExpiredRemindAts(ctx context.Context) {
	tasks, err := s.q.ListTasks(ctx, store.ListTasksParams{})
	if err != nil {
		s.logger().Warn("读取任务列表失败，跳过过期清理", "error", err)
		return
	}
	now := s.now()
	for _, task := range tasks {
		if task.IsDaily || !task.RemindAt.Valid || task.DeletedAt.Valid {
			continue
		}
		if now.Before(task.RemindAt.Time.Add(oneshotGrace)) {
			continue
		}
		if err := s.clearRemindAt(ctx, task.ID); err != nil {
			s.logger().Warn("清理过期 remind_at 失败", "task_id", task.ID, "error", err)
		}
	}
}

func (s *Scheduler) clearRemindAt(ctx context.Context, id int64) error {
	task, err := s.q.GetTask(ctx, id)
	if err != nil {
		return err
	}
	if !task.RemindAt.Valid {
		return nil
	}
	_, err = s.q.UpdateTask(ctx, store.UpdateTaskParams{
		Title:             task.Title,
		Description:       task.Description,
		Status:            task.Status,
		Priority:          task.Priority,
		DueDate:           task.DueDate,
		Tags:              task.Tags,
		IsDaily:           task.IsDaily,
		LastCompletedDate: task.LastCompletedDate,
		ReminderTime:      task.ReminderTime,
		RemindAt:          pgtype.Timestamptz{},
		ID:                task.ID,
	})
	return err
}

func (s *Scheduler) sendSync(ctx context.Context, msg Message) {
	if s.mailer == nil || !s.mailer.Enabled() {
		s.logger().Debug("邮件发送器未启用，跳过发送", "subject", msg.Subject)
		return
	}
	if err := s.mailer.Send(ctx, msg); err != nil {
		s.logger().Error("邮件发送失败", "subject", msg.Subject, "error", err)
	}
}

func (s *Scheduler) register(category string, items []scheduled) {
	now := s.now()

	s.mu.Lock()
	defer s.mu.Unlock()

	for _, timer := range s.timers[category] {
		timer.Stop()
	}
	built := make([]*time.Timer, 0, len(items))
	for _, item := range items {
		delay := max(item.fire.Sub(now), 0)
		built = append(built, time.AfterFunc(delay, item.fn))
	}
	s.timers[category] = built
}

func (s *Scheduler) stopAll() {
	s.mu.Lock()
	defer s.mu.Unlock()
	for category := range s.timers {
		for _, timer := range s.timers[category] {
			timer.Stop()
		}
		delete(s.timers, category)
	}
}

func (s *Scheduler) wrapped(ctx context.Context, fn func()) func() {
	return func() {
		defer func() {
			if r := recover(); r != nil {
				s.logger().Error("调度回调 panic", "panic", r)
			}
		}()
		if ctx != nil && ctx.Err() != nil {
			return
		}
		fn()
	}
}

func (s *Scheduler) baseCtx() context.Context {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.ctx != nil {
		return s.ctx
	}
	return context.Background()
}

func (s *Scheduler) now() time.Time {
	if s.Now == nil {
		return time.Now()
	}
	return s.Now()
}

func (s *Scheduler) logger() *slog.Logger {
	if s == nil || s.log == nil {
		return slog.Default()
	}
	return s.log
}

type reminderKind int

const (
	reminderDaily reminderKind = iota
	reminderOneShot
)

type reminder struct {
	ID    int64
	Title string
	Kind  reminderKind
	When  time.Time
}

func (r reminder) nextOccurrence(now time.Time) time.Time {
	if r.Kind == reminderDaily {
		return nextDailyOccurrence(now, r.When)
	}
	return r.When
}

func (r reminder) isExpired(now time.Time) bool {
	if r.Kind != reminderOneShot {
		return false
	}
	return !now.Before(r.When.Add(oneshotGrace))
}

func remindersFromTasks(tasks []store.Task) []reminder {
	reminders := make([]reminder, 0, len(tasks))
	for _, task := range tasks {
		if task.DeletedAt.Valid {
			continue
		}
		if task.IsDaily {
			hour, minute, ok := reminderClock(task.ReminderTime)
			if !ok {
				continue
			}
			reminders = append(reminders, reminder{
				ID:    task.ID,
				Title: task.Title,
				Kind:  reminderDaily,
				When:  time.Date(2000, 1, 1, hour, minute, 0, 0, chinaTZ()),
			})
			continue
		}
		if !task.RemindAt.Valid {
			continue
		}
		reminders = append(reminders, reminder{
			ID:    task.ID,
			Title: task.Title,
			Kind:  reminderOneShot,
			When:  task.RemindAt.Time,
		})
	}
	return reminders
}

// nextTaskBatch returns the earliest upcoming reminder instant and every active
// reminder due at that instant. ok=false when nothing is schedulable, in which
// case the caller re-checks after taskRetryInterval.
//
// This intentionally deviates from Rust daily_reminder.rs::plan_next, which
// returned the due set only when wait==0: that loop captured the set right
// before the target via integer-second truncation and masked repeats with OS
// notification ids. Email has no such dedup and the Go scheduler re-plans after
// its timer fires, so the due set is captured here for the exact fire instant.
// After firing, one-shots are cleared in the DB and dailies roll to the next
// day, giving exactly-once delivery without a sub-second refire.
func nextTaskBatch(reminders []reminder, now time.Time) (time.Time, []reminder, bool) {
	active := make([]reminder, 0, len(reminders))
	for _, r := range reminders {
		if r.isExpired(now) {
			continue
		}
		active = append(active, r)
	}
	if len(active) == 0 {
		return time.Time{}, nil, false
	}

	fire := active[0].nextOccurrence(now)
	for _, r := range active[1:] {
		if occurrence := r.nextOccurrence(now); occurrence.Before(fire) {
			fire = occurrence
		}
	}

	due := make([]reminder, 0, len(active))
	for _, r := range active {
		if r.nextOccurrence(now).Equal(fire) {
			due = append(due, r)
		}
	}
	return fire, due, true
}

func nextDailyOccurrence(now, when time.Time) time.Time {
	local := now.In(chinaTZ())
	target := time.Date(local.Year(), local.Month(), local.Day(), when.Hour(), when.Minute(), 0, 0, chinaTZ())
	if !target.Before(local) {
		return target
	}
	return target.AddDate(0, 0, 1)
}

func nextSevenAM(now time.Time) time.Time {
	local := now.In(chinaTZ())
	seven := time.Date(local.Year(), local.Month(), local.Day(), 7, 0, 0, 0, chinaTZ())
	if local.Before(seven) {
		return seven
	}
	return seven.AddDate(0, 0, 1)
}

func reminderClock(clock pgtype.Time) (int, int, bool) {
	if !clock.Valid || clock.Microseconds < 0 {
		return 0, 0, false
	}
	seconds := clock.Microseconds / 1000000
	if seconds < 0 {
		return 0, 0, false
	}
	hour := int(seconds / 3600)
	minute := int((seconds % 3600) / 60)
	if hour > 23 || minute > 59 {
		return 0, 0, false
	}
	return hour, minute, true
}

func parseExamOffsets(raw []byte) ([]int, error) {
	if len(bytes.TrimSpace(raw)) == 0 {
		return nil, errEmptyOffsets
	}
	var offsets []int
	if err := json.Unmarshal(raw, &offsets); err != nil {
		return nil, err
	}
	if offsets == nil {
		return nil, errEmptyOffsets
	}
	return offsets, nil
}

func chinaTZ() *time.Location {
	return time.FixedZone("CST", 8*3600)
}

var _ pomodoro.Notifier = (*Scheduler)(nil)
