/** 与后端 lzu::models::LzuProfileSummary 对齐（src-tauri/src/lzu/models.rs）。 */
export interface LzuProfileSummary {
  /** 姓名，用于登录身份确认。 */
  display_name: string | null
  /** 人员编号；资料缺失时后端回退到登录用户名。 */
  person_no: string | null
  /** 学院、部门或单位名称。 */
  department: string | null
  /** 人员类别。 */
  role: string | null
  /** 校园卡号后 4 位；后端不会返回完整卡号。 */
  campus_card_tail: string | null
}

/** 与后端 lzu::models::AuthStatus 对齐（src-tauri/src/lzu/models.rs）。 */
export interface LzuAuthStatus {
  /** 是否已登录。 */
  is_logged_in: boolean
  /** 登录用户名，未登录时为 null。 */
  username: string | null
  /** 当前登录账号的低敏身份摘要；未登录或资料获取失败时为 null。 */
  profile: LzuProfileSummary | null
}

/** 与后端 commands::lzu::LzuCourseImportResult 对齐（src-tauri/src/commands/lzu.rs）。 */
export interface LzuCourseImportResult {
  /** 从 LZU API 拉取到的课程记录数。 */
  parsed: number
  /** 实际写入数据库的记录数。 */
  imported: number
  /** 因去重跳过的记录数。 */
  skipped: number
  /** 无法映射的课程记录数。 */
  failed: number
  /** 后端生成的中文摘要。 */
  message: string
}

/** 与后端 lzu::easytong::CampusWallet 对齐。 */
export interface LzuCampusWallet {
  card_name: string | null
  unit: string | null
  wallet_money: string | null
  wallet_name: string | null
  is_withdraw: string | null
  money_max: string | null
  mon_temp: string | null
  mon_card: string | null
  wallet_num: string | null
}

/** 与后端 lzu::easytong::CampusCardAccount 对齐。 */
export interface LzuCampusCardAccount {
  card_tail: string | null
  epid_available: boolean
}

/** 与后端 lzu::easytong::CampusCardOverview 对齐。 */
export interface LzuCampusCardOverview {
  account: LzuCampusCardAccount
  wallets: LzuCampusWallet[]
}

/** 与后端 lzu::services::LzuServiceItem 对齐。 */
export interface LzuServiceItem {
  id: string | null
  name: string
  icon_url: string | null
  category_name: string | null
  introduce: string | null
  requires_login: boolean
  is_new: boolean
  is_top: boolean
  is_hot: boolean
  sort: number
}

/** 与后端 lzu::services::LzuServiceCategory 对齐。 */
export interface LzuServiceCategory {
  id: string | null
  name: string
  icon_url: string | null
  sort: number
  services: LzuServiceItem[]
}

/** 与后端 lzu::services::LzuServiceDirectory 对齐。 */
export interface LzuServiceDirectory {
  categories: LzuServiceCategory[]
}
