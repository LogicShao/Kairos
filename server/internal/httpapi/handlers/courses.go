package handlers

import (
	"log/slog"
	"net/http"

	"github.com/jackc/pgx/v5/pgtype"

	"kairos/server/internal/domain/importer"
	"kairos/server/internal/httpapi/dto"
	"kairos/server/internal/store"
)

// Courses implements the /api/courses endpoints.
type Courses struct {
	Q   *store.Queries
	Log *slog.Logger
}

// List handles GET /api/courses?semester=.
func (h *Courses) List(w http.ResponseWriter, r *http.Request) {
	semester := pgtype.Text{}
	if v := r.URL.Query().Get("semester"); v != "" {
		semester = store.TextOf(v)
	}
	courses, err := h.Q.ListCourses(r.Context(), semester)
	if err != nil {
		h.Log.Error("list courses", "error", err)
		writeError(w, http.StatusInternalServerError, "internal server error")
		return
	}
	out := make([]dto.Course, 0, len(courses))
	for _, c := range courses {
		out = append(out, dto.FromCourse(c))
	}
	writeJSON(w, http.StatusOK, out)
}

// Create handles POST /api/courses.
func (h *Courses) Create(w http.ResponseWriter, r *http.Request) {
	var req dto.CreateCourseRequest
	if !decodeJSON(w, r, &req) {
		return
	}
	if req.Name == "" {
		writeError(w, http.StatusBadRequest, "name is required")
		return
	}
	if req.DayOfWeek < 1 || req.DayOfWeek > 7 {
		writeError(w, http.StatusBadRequest, "day_of_week must be between 1 and 7")
		return
	}
	startTime, err := dto.ParseClock(&req.StartTime)
	if err != nil {
		writeError(w, http.StatusBadRequest, err.Error())
		return
	}
	endTime, err := dto.ParseClock(&req.EndTime)
	if err != nil {
		writeError(w, http.StatusBadRequest, err.Error())
		return
	}
	semesterStartDate, err := dto.ParseDate(req.SemesterStartDate)
	if err != nil {
		writeError(w, http.StatusBadRequest, err.Error())
		return
	}

	course, err := h.Q.CreateCourse(r.Context(), store.CreateCourseParams{
		SyncID:            "",
		Name:              req.Name,
		DayOfWeek:         req.DayOfWeek,
		StartTime:         startTime,
		EndTime:           endTime,
		WeekPattern:       strOr(req.WeekPattern, ""),
		SemesterStartDate: semesterStartDate,
		Location:          strOr(req.Location, ""),
		Teacher:           strOr(req.Teacher, ""),
		Color:             strOr(req.Color, "#7C8CC0"),
		Semester:          strOr(req.Semester, ""),
	})
	if err != nil {
		h.Log.Error("create course", "error", err)
		writeError(w, http.StatusInternalServerError, "internal server error")
		return
	}
	writeJSON(w, http.StatusCreated, dto.FromCourse(course))
}

// Update handles PATCH /api/courses/{id}.
func (h *Courses) Update(w http.ResponseWriter, r *http.Request) {
	id, ok := pathID(w, r)
	if !ok {
		return
	}
	var req dto.UpdateCourseRequest
	if !decodeJSON(w, r, &req) {
		return
	}

	existing, err := h.Q.GetCourse(r.Context(), id)
	if err != nil {
		writeStoreError(w, err)
		return
	}

	name := existing.Name
	if req.Name != nil {
		name = *req.Name
	}
	if name == "" {
		writeError(w, http.StatusBadRequest, "name is required")
		return
	}
	dayOfWeek := existing.DayOfWeek
	if req.DayOfWeek != nil {
		dayOfWeek = *req.DayOfWeek
	}
	if dayOfWeek < 1 || dayOfWeek > 7 {
		writeError(w, http.StatusBadRequest, "day_of_week must be between 1 and 7")
		return
	}
	startTime := existing.StartTime
	if req.StartTime != nil {
		parsed, err := dto.ParseClock(req.StartTime)
		if err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}
		startTime = parsed
	}
	endTime := existing.EndTime
	if req.EndTime != nil {
		parsed, err := dto.ParseClock(req.EndTime)
		if err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}
		endTime = parsed
	}
	semesterStartDate := existing.SemesterStartDate
	if req.SemesterStartDate != nil {
		parsed, err := dto.ParseDate(req.SemesterStartDate)
		if err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}
		semesterStartDate = parsed
	}

	updated, err := h.Q.UpdateCourse(r.Context(), store.UpdateCourseParams{
		Name:              name,
		DayOfWeek:         dayOfWeek,
		StartTime:         startTime,
		EndTime:           endTime,
		WeekPattern:       strOr(req.WeekPattern, existing.WeekPattern),
		SemesterStartDate: semesterStartDate,
		Location:          strOr(req.Location, existing.Location),
		Teacher:           strOr(req.Teacher, existing.Teacher),
		Color:             strOr(req.Color, existing.Color),
		Semester:          strOr(req.Semester, existing.Semester),
		ID:                id,
	})
	if err != nil {
		writeStoreError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, dto.FromCourse(updated))
}

// Delete handles DELETE /api/courses/{id} (soft delete).
func (h *Courses) Delete(w http.ResponseWriter, r *http.Request) {
	id, ok := pathID(w, r)
	if !ok {
		return
	}
	if err := h.Q.SoftDeleteCourse(r.Context(), id); err != nil {
		writeStoreError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

// ImportText handles POST /api/courses/import-text.
func (h *Courses) ImportText(w http.ResponseWriter, r *http.Request) {
	var req dto.ImportCoursesRequest
	if !decodeJSON(w, r, &req) {
		return
	}
	candidates, err := importer.ParseCourseImportText(req.Text, req.Semester, req.SemesterStartDate)
	if err != nil {
		writeError(w, http.StatusBadRequest, err.Error())
		return
	}
	result, err := importer.ImportNewCourses(r.Context(), h.Q, candidates, req.Semester)
	if err != nil {
		h.Log.Error("import courses", "error", err)
		writeError(w, http.StatusInternalServerError, "internal server error")
		return
	}
	writeJSON(w, http.StatusOK, dto.ImportTextResult{
		Parsed:   result.Parsed,
		Imported: result.Imported,
		Skipped:  result.Skipped,
		Message:  result.Message,
	})
}

// ResetSemesterDates handles POST /api/courses/reset-semester-dates.
func (h *Courses) ResetSemesterDates(w http.ResponseWriter, r *http.Request) {
	var req dto.ResetSemesterDatesRequest
	if !decodeJSON(w, r, &req) {
		return
	}
	date, err := dto.ParseDate(&req.Date)
	if err != nil {
		writeError(w, http.StatusBadRequest, err.Error())
		return
	}
	courseCount, err := h.Q.UpdateAllCoursesSemesterStartDate(r.Context(), date)
	if err != nil {
		h.Log.Error("reset course semester dates", "error", err)
		writeError(w, http.StatusInternalServerError, "internal server error")
		return
	}
	if _, err := h.Q.UpdateAllSemesterContextsStartDate(r.Context(), date); err != nil {
		h.Log.Error("reset semester context dates", "error", err)
		writeError(w, http.StatusInternalServerError, "internal server error")
		return
	}
	writeJSON(w, http.StatusOK, dto.ResetSemesterDatesResponse{Updated: courseCount})
}

func strOr(p *string, fallback string) string {
	if p == nil {
		return fallback
	}
	return *p
}
