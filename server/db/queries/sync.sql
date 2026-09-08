-- name: GetSyncConfig :one
SELECT * FROM sync_config
WHERE id = 1;

-- name: UpdateSyncConfigCredentials :exec
UPDATE sync_config
SET server_url = @server_url,
    username = @username,
    password = @password,
    auto_sync = @auto_sync
WHERE id = 1;

-- name: UpdateSyncLastSyncAt :exec
UPDATE sync_config
SET last_sync_at = @last_sync_at
WHERE id = 1;

-- name: UpdateSyncRemoteEtag :exec
UPDATE sync_config
SET remote_etag = @remote_etag
WHERE id = 1;

-- name: UpdateSyncAiSettingsRemoteEtag :exec
UPDATE sync_config
SET ai_settings_remote_etag = @ai_settings_remote_etag
WHERE id = 1;

-- name: EnsureSyncDeviceIds :execrows
UPDATE sync_config
SET device_id = CASE WHEN device_id IS NULL OR device_id = '' THEN @device_id ELSE device_id END,
    dataset_id = CASE WHEN dataset_id IS NULL OR dataset_id = '' THEN @dataset_id ELSE dataset_id END
WHERE id = 1;
