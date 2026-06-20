/** 与后端 importers::ImportTextResult 对齐（src-tauri/src/importers.rs）。 */
export interface ImportTextResult {
  parsed: number
  imported: number
  skipped: number
  message: string
}
