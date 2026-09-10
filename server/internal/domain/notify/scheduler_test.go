package notify

import (
	"testing"
	"time"

	"github.com/jackc/pgx/v5/pgtype"

	"kairos/server/internal/store"
)

func cst(year int, month time.Month, day, hour, minute, second int) time.Time {
	return time.Date(year, month, day, hour, minute, second, 0, chinaTZ())
}

func dailyReminder(id int64, title string, hour, minute int) reminder {
	return reminder{
		ID:    id,
		Title: title,
		Kind:  reminderDaily,
		When:  time.Date(2000, 1, 1, hour, minute, 0, 0, chinaTZ()),
	}
}

func oneShotReminder(id int64, title string, when time.Time) reminder {
	return reminder{ID: id, Title: title, Kind: reminderOneShot, When: when}
}

func TestNextDailyOccurrenceSameDayAndNextDay(t *testing.T) {
	now := cst(2026, time.August, 1, 9, 0, 0)

	if got, want := nextDailyOccurrence(now, cst(2000, 1, 1, 10, 0, 0)), cst(2026, time.August, 1, 10, 0, 0); !got.Equal(want) {
		t.Errorf("same day: got %v, want %v", got, want)
	}
	if got, want := nextDailyOccurrence(now, cst(2000, 1, 1, 8, 0, 0)), cst(2026, time.August, 2, 8, 0, 0); !got.Equal(want) {
		t.Errorf("next day: got %v, want %v", got, want)
	}
}

func TestNextTaskBatchDailySingle(t *testing.T) {
	now := cst(2026, time.August, 1, 9, 0, 0)
	fire, due, ok := nextTaskBatch([]reminder{dailyReminder(1, "背单词", 20, 0)}, now)
	if !ok {
		t.Fatal("ok = false, want true")
	}
	if want := cst(2026, time.August, 1, 20, 0, 0); !fire.Equal(want) {
		t.Errorf("fire = %v, want %v", fire, want)
	}
	if len(due) != 1 || due[0].ID != 1 || due[0].Kind != reminderDaily {
		t.Fatalf("due = %v, want one daily reminder id 1", due)
	}
}

func TestNextTaskBatchDailyDueAtNow(t *testing.T) {
	now := cst(2026, time.August, 1, 20, 0, 0)
	fire, due, ok := nextTaskBatch([]reminder{dailyReminder(1, "背单词", 20, 0)}, now)
	if !ok {
		t.Fatal("ok = false, want true")
	}
	if !fire.Equal(now) {
		t.Errorf("fire = %v, want %v", fire, now)
	}
	if len(due) != 1 || due[0].ID != 1 {
		t.Fatalf("due = %v, want one reminder id 1", due)
	}
}

func TestNextTaskBatchDailySameTimeMultiple(t *testing.T) {
	reminders := []reminder{
		dailyReminder(1, "A", 8, 0),
		dailyReminder(2, "B", 8, 0),
		dailyReminder(3, "C", 9, 0),
	}

	fire, due, ok := nextTaskBatch(reminders, cst(2026, time.August, 1, 7, 0, 0))
	if !ok {
		t.Fatal("ok = false, want true")
	}
	if want := cst(2026, time.August, 1, 8, 0, 0); !fire.Equal(want) {
		t.Errorf("fire = %v, want %v", fire, want)
	}
	if len(due) != 2 {
		t.Fatalf("due = %v, want both 08:00 reminders", due)
	}
}

func TestNextTaskBatchEmpty(t *testing.T) {
	if _, _, ok := nextTaskBatch(nil, cst(2026, time.August, 1, 7, 0, 0)); ok {
		t.Error("ok = true, want false for empty reminders")
	}
}

func TestNextTaskBatchOneShotFuture(t *testing.T) {
	now := cst(2026, time.August, 3, 9, 0, 0)
	when := cst(2026, time.August, 5, 14, 0, 0)
	fire, due, ok := nextTaskBatch([]reminder{oneShotReminder(1, "交报告", when)}, now)
	if !ok {
		t.Fatal("ok = false, want true")
	}
	if !fire.Equal(when) {
		t.Errorf("fire = %v, want %v", fire, when)
	}
	if len(due) != 1 || due[0].ID != 1 {
		t.Fatalf("due = %v, want one reminder id 1", due)
	}
}

func TestNextTaskBatchOneShotDueWithinGrace(t *testing.T) {
	now := cst(2026, time.August, 3, 14, 0, 30)
	when := cst(2026, time.August, 3, 14, 0, 0)
	fire, due, ok := nextTaskBatch([]reminder{oneShotReminder(1, "交报告", when)}, now)
	if !ok {
		t.Fatal("ok = false, want true")
	}
	if fire.After(now) {
		t.Errorf("fire = %v, want <= now %v", fire, now)
	}
	if len(due) != 1 || due[0].ID != 1 || due[0].Kind != reminderOneShot {
		t.Fatalf("due = %v, want one one-shot reminder id 1", due)
	}
}

func TestNextTaskBatchOneShotExpiredExcluded(t *testing.T) {
	now := cst(2026, time.August, 3, 14, 5, 0)
	when := cst(2026, time.August, 3, 14, 0, 0)
	if _, _, ok := nextTaskBatch([]reminder{oneShotReminder(1, "交报告", when)}, now); ok {
		t.Error("ok = true, want false for expired one-shot only")
	}
}

func TestNextTaskBatchMixedDailyAndOneShot(t *testing.T) {
	reminders := []reminder{
		dailyReminder(1, "背单词", 20, 0),
		oneShotReminder(2, "交报告", cst(2026, time.August, 3, 10, 0, 0)),
	}

	fire, due, ok := nextTaskBatch(reminders, cst(2026, time.August, 3, 9, 0, 0))
	if !ok {
		t.Fatal("ok = false, want true")
	}
	if want := cst(2026, time.August, 3, 10, 0, 0); !fire.Equal(want) {
		t.Errorf("fire = %v, want %v", fire, want)
	}
	if len(due) != 1 || due[0].ID != 2 {
		t.Fatalf("due = %v, want only the one-shot id 2", due)
	}

	fire, due, ok = nextTaskBatch(reminders, cst(2026, time.August, 3, 10, 0, 0))
	if !ok {
		t.Fatal("ok at due = false, want true")
	}
	if !fire.Equal(cst(2026, time.August, 3, 10, 0, 0)) {
		t.Errorf("fire at due = %v, want 10:00", fire)
	}
	if len(due) != 1 || due[0].ID != 2 {
		t.Fatalf("due at 10:00 = %v, want one reminder id 2", due)
	}
}

func TestNextTaskBatchDailyBoundaryNoSameDayRefire(t *testing.T) {
	daily := dailyReminder(1, "背单词", 20, 0)

	before := cst(2026, time.August, 1, 19, 59, 59).Add(500 * time.Millisecond)
	fire, due, ok := nextTaskBatch([]reminder{daily}, before)
	if !ok {
		t.Fatal("ok before boundary = false, want true")
	}
	if want := cst(2026, time.August, 1, 20, 0, 0); !fire.Equal(want) {
		t.Errorf("fire before boundary = %v, want %v", fire, want)
	}
	if len(due) != 1 || due[0].ID != 1 {
		t.Fatalf("due before boundary = %v, want one daily id 1", due)
	}

	after := cst(2026, time.August, 1, 20, 0, 0).Add(time.Millisecond)
	fire, due, ok = nextTaskBatch([]reminder{daily}, after)
	if !ok {
		t.Fatal("ok after boundary = false, want true")
	}
	if want := cst(2026, time.August, 2, 20, 0, 0); !fire.Equal(want) {
		t.Errorf("fire after boundary = %v, want tomorrow %v (no same-day refire)", fire, want)
	}
	if len(due) != 1 || due[0].ID != 1 {
		t.Fatalf("due after boundary = %v, want one daily id 1", due)
	}
}

func TestNextSevenAMBoundary(t *testing.T) {
	before := cst(2026, time.July, 31, 6, 59, 59)
	if got, want := nextSevenAM(before), cst(2026, time.July, 31, 7, 0, 0); !got.Equal(want) {
		t.Errorf("before 07:00: got %v, want %v", got, want)
	}
	if got := nextSevenAM(before).Sub(before); got != time.Second {
		t.Errorf("before wait = %v, want 1s", got)
	}

	at := cst(2026, time.July, 31, 7, 0, 0)
	if got, want := nextSevenAM(at), cst(2026, time.August, 1, 7, 0, 0); !got.Equal(want) {
		t.Errorf("at 07:00: got %v, want %v", got, want)
	}
	if got := nextSevenAM(at).Sub(at); got != 24*time.Hour {
		t.Errorf("at wait = %v, want 24h", got)
	}
}

func TestNextSevenAMNormalizesLocation(t *testing.T) {
	nowUTC := time.Date(2026, time.July, 31, 22, 0, 0, 0, time.UTC)
	if got, want := nextSevenAM(nowUTC), cst(2026, time.August, 1, 7, 0, 0); !got.Equal(want) {
		t.Errorf("UTC input: got %v, want %v", got, want)
	}
}

func TestParseExamOffsets(t *testing.T) {
	offsets, err := parseExamOffsets([]byte("[1440,60]"))
	if err != nil {
		t.Fatalf("valid offsets error: %v", err)
	}
	if len(offsets) != 2 || offsets[0] != 1440 || offsets[1] != 60 {
		t.Fatalf("offsets = %v, want [1440 60]", offsets)
	}

	if offsets, err := parseExamOffsets([]byte("[]")); err != nil || len(offsets) != 0 {
		t.Errorf("empty array = %v, %v; want empty, nil", offsets, err)
	}
	for _, raw := range []string{"", "   ", "not-json", "null"} {
		if _, err := parseExamOffsets([]byte(raw)); err == nil {
			t.Errorf("parseExamOffsets(%q) = nil error, want error", raw)
		}
	}
}

func TestReminderClock(t *testing.T) {
	clock := pgtype.Time{Microseconds: int64(20*3600) * 1000000, Valid: true}
	hour, minute, ok := reminderClock(clock)
	if !ok || hour != 20 || minute != 0 {
		t.Errorf("reminderClock = %d:%02d,%v; want 20:00,true", hour, minute, ok)
	}
	if _, _, ok := reminderClock(pgtype.Time{}); ok {
		t.Error("invalid clock should report ok=false")
	}
}

func TestRemindersFromTasks(t *testing.T) {
	tasks := []store.Task{
		{
			ID:           1,
			Title:        "背单词",
			IsDaily:      true,
			ReminderTime: pgtype.Time{Microseconds: int64(20*3600) * 1000000, Valid: true},
		},
		{
			ID:       2,
			Title:    "交报告",
			IsDaily:  false,
			RemindAt: pgtype.Timestamptz{Time: cst(2026, time.August, 5, 14, 0, 0), Valid: true},
		},
		{
			ID:        3,
			Title:     "已删除",
			IsDaily:   false,
			RemindAt:  pgtype.Timestamptz{Time: cst(2026, time.August, 5, 14, 0, 0), Valid: true},
			DeletedAt: pgtype.Timestamptz{Time: cst(2026, time.August, 1, 0, 0, 0), Valid: true},
		},
		{
			ID:      4,
			Title:   "无时间",
			IsDaily: true,
		},
		{
			ID:      5,
			Title:   "无提醒",
			IsDaily: false,
		},
	}

	reminders := remindersFromTasks(tasks)
	if len(reminders) != 2 {
		t.Fatalf("reminders = %v, want 2", reminders)
	}
	if reminders[0].Kind != reminderDaily || reminders[0].ID != 1 {
		t.Errorf("first reminder = %+v, want daily id 1", reminders[0])
	}
	if reminders[0].When.Hour() != 20 || reminders[0].When.Minute() != 0 {
		t.Errorf("daily clock = %v, want 20:00", reminders[0].When)
	}
	if reminders[1].Kind != reminderOneShot || reminders[1].ID != 2 {
		t.Errorf("second reminder = %+v, want one-shot id 2", reminders[1])
	}
}
