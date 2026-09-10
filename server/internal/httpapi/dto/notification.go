package dto

import (
	"kairos/server/internal/store"
)

// NotificationConfig mirrors the frontend src/types/notification.ts. The
// exam_offsets_json field carries the DB jsonb bytes as a JSON array string.
type NotificationConfig struct {
	ID                    int64  `json:"id"`
	Enabled               bool   `json:"enabled"`
	ExamOffsetsJSON       string `json:"exam_offsets_json"`
	AndroidChannelCreated bool   `json:"android_channel_created"`
	CreatedAt             string `json:"created_at"`
	UpdatedAt             string `json:"updated_at"`
}

// UpdateNotificationConfigRequest is the body of PATCH /api/notify/config.
// Omitted fields keep their stored value.
type UpdateNotificationConfigRequest struct {
	Enabled               *bool   `json:"enabled"`
	ExamOffsetsJSON       *string `json:"exam_offsets_json"`
	AndroidChannelCreated *bool   `json:"android_channel_created"`
}

// FromNotificationConfig converts a store row into the API representation.
// android_channel_created is always false: the web build has no Android
// notification channel and the value is never persisted.
func FromNotificationConfig(c store.NotificationConfig) NotificationConfig {
	return NotificationConfig{
		ID:                    c.ID,
		Enabled:               c.Enabled,
		ExamOffsetsJSON:       string(c.ExamOffsets),
		AndroidChannelCreated: false,
		CreatedAt:             store.TSString(c.CreatedAt),
		UpdatedAt:             store.TSString(c.UpdatedAt),
	}
}
