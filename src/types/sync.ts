export interface SyncConfig {
  id: number
  server_url: string
  username: string
  password: string
  auto_sync: boolean
  last_sync_at: string | null
  remote_etag: string | null
  device_id: string | null
  dataset_id: string | null
}

export interface SyncStats {
  tasks_merged: number
  courses_merged: number
  exams_merged: number
  sessions_merged: number
  conflicts: number
}

export interface SyncResult {
  uploaded: boolean
  downloaded: boolean
  stats: SyncStats
}
