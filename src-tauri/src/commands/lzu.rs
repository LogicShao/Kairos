//! LZU 相关 Tauri commands — 薄命令层。
//!
//! 业务逻辑委托给 `crate::lzu` 模块，本模块只做参数传递和状态获取。

use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::Serialize;
use tauri::{Emitter, State};

use crate::lzu::appservice::AppServiceClient;
use crate::lzu::auth::SharedLzuAuth;
use crate::lzu::easytong::{
    campus_card_overview, CampusCardOverview, EasyTongClient, EasyTongSession,
};
use crate::lzu::error::LzuError;
use crate::lzu::models::{AuthStatus, LoginRequest, LzuProfileSummary, LzuSession, XlxxData};
use crate::lzu::services::{sanitize_service_directory, LzuServiceDirectory};

const FALLBACK_TOTAL_WEEKS: i64 = 24;

#[derive(Debug, Clone, Serialize)]
pub struct LzuCourseImportResult {
    pub parsed: usize,
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub message: String,
}

fn require_lzu_clients(
    lzu_auth: &SharedLzuAuth,
) -> Result<(Arc<AppServiceClient>, Arc<EasyTongClient>, LzuSession), String> {
    let auth = lzu_auth.lock().map_err(|e| format!("内部错误: {e}"))?;
    let session = auth
        .session
        .clone()
        .ok_or_else(|| LzuError::NotLoggedIn.to_string())?;
    Ok((auth.client.clone(), auth.easytong_client.clone(), session))
}

fn api_error(code: i64, message: String) -> String {
    LzuError::Api { code, message }.to_string()
}

/// 清除登录态（内存 + SQLite），用于 token 过期等异常场景。
fn clear_session(lzu_auth: &SharedLzuAuth, db: &Arc<Mutex<Connection>>) {
    if let Ok(mut auth) = lzu_auth.lock() {
        auth.session = None;
        match db.lock() {
            Ok(conn) => {
                if let Err(e) = auth.persist(&conn) {
                    log::error!("LZU session clear persist failed: {e}");
                }
            }
            Err(e) => log::error!("failed to lock DB for LZU session clear: {e}"),
        }
    }
}

/// API code 校验：`code != 1` 视为登录态失效，清会话并返回错误。
fn fail_with_session_clear(
    lzu_auth: &SharedLzuAuth,
    db: &Arc<Mutex<Connection>>,
    code: i64,
    message: String,
) -> Result<(), String> {
    if code != 1 {
        clear_session(lzu_auth, db);
        return Err(api_error(code, message));
    }
    Ok(())
}

/// 请求失败时清除本地会话并返回错误文案，供 `map_err` 使用。
fn clear_on_err(lzu_auth: &SharedLzuAuth, db: &Arc<Mutex<Connection>>, msg: String) -> String {
    clear_session(lzu_auth, db);
    msg
}

/// 拉取 LZU 学期信息（xlxx）。`code != 1` 或缺 data 均返回错误。
async fn fetch_xlxx(client: &AppServiceClient, session: &LzuSession) -> Result<XlxxData, String> {
    let response = client
        .get_xlxx(&session.gateway_token)
        .await
        .map_err(|e| e.to_string())?;
    if response.code != 1 {
        return Err(api_error(response.code, response.message));
    }
    response
        .data
        .ok_or_else(|| "LZU 学期信息响应缺少 data 字段".to_string())
}

fn schedule_week_limit(xlxx: &XlxxData) -> Result<(i64, bool), String> {
    if let Some(raw) = xlxx
        .zzx
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok((
            crate::lzu::mapper::parse_positive_week(raw, "总周次 zzx")?,
            true,
        ));
    }

    let current_week = xlxx
        .dqrqszzc
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| crate::lzu::mapper::parse_positive_week(value, "当前周次 dqrqszzc"))
        .transpose()?
        .unwrap_or(1);
    let total_weeks = FALLBACK_TOTAL_WEEKS.max(current_week);
    log::warn!("LZU 学期信息缺少 zzx，按 {total_weeks} 周尝试拉取课表");
    Ok((total_weeks, false))
}

fn persist_lzu_semester_context(
    conn: &Connection,
    xlxx: &XlxxData,
    semester_hint: Option<&str>,
    default_total_weeks: i64,
) -> Result<(), String> {
    let req = match crate::lzu::mapper::map_semester_context(xlxx, semester_hint) {
        Ok(req) => req,
        Err(err) => {
            log::warn!("LZU 学期上下文未保存: {err}");
            return Ok(());
        }
    };

    crate::db::semester::upsert_semester_context(conn, &req)
        .map(|_| ())
        .map_err(|e| e.to_string())?;

    let total_weeks = req.total_weeks.unwrap_or(default_total_weeks);
    crate::db::term_phases::ensure_default_phases(conn, &req.term_label, total_weeks)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

async fn fetch_profile_summary(
    client: &AppServiceClient,
    login_token: &str,
    gateway_token: &str,
    fallback_username: &str,
) -> Result<Option<LzuProfileSummary>, ()> {
    match client.user_info(login_token, gateway_token).await {
        Ok(response) if response.code == 1 => match response.data {
            Some(data) => Ok(Some(LzuProfileSummary::from_user_info(
                &data,
                fallback_username,
            ))),
            None => {
                log::warn!("LZU 用户资料响应缺少 data 字段，已跳过身份摘要");
                Ok(None)
            }
        },
        Ok(response) => {
            log::warn!(
                "LZU 用户资料接口返回异常，已跳过身份摘要: code={}",
                response.code
            );
            Err(())
        }
        Err(_) => {
            log::warn!("LZU 用户资料拉取失败，已跳过身份摘要");
            Err(())
        }
    }
}

async fn refresh_easytong_session(
    client: &AppServiceClient,
    easytong_client: &EasyTongClient,
    session: &LzuSession,
) -> Result<(EasyTongSession, crate::lzu::easytong::EasyTongAccountInfo), String> {
    let st_response = client
        .get_st(&session.login_token, "")
        .await
        .map_err(|e| e.to_string())?;
    if st_response.code != 1 {
        return Err(api_error(st_response.code, st_response.message));
    }
    let st = st_response
        .data
        .ok_or_else(|| "getSt 响应缺少 data 字段".to_string())?;

    let mut easytong_session = easytong_client
        .exchange_et_token(&st)
        .await
        .map_err(|e| e.to_string())?;
    let account = easytong_client
        .get_acc_info(&easytong_session)
        .await
        .map_err(|e| e.to_string())?;
    if account.code != 1 {
        return Err(api_error(account.code, account.msg.clone()));
    }

    easytong_session.card_acc_num = account.card_acc_num.clone();
    easytong_session.epid = account.epid.clone();
    Ok((easytong_session, account))
}

/// 登录 LZU 统一认证。
///
/// 返回登录状态摘要（不含 token 原文）。
#[tauri::command]
pub async fn lzu_login(
    app_handle: tauri::AppHandle,
    db: State<'_, Arc<Mutex<Connection>>>,
    lzu_auth: State<'_, SharedLzuAuth>,
    username: String,
    password: String,
) -> Result<AuthStatus, String> {
    // 1. 从锁中取出 client（clone 是廉价操作，reqwest::Client 内部是 Arc）
    let client = {
        let auth = lzu_auth.lock().map_err(|e| format!("内部错误: {e}"))?;
        auth.client.clone()
    };

    // 2. 构建登录请求
    let request = LoginRequest {
        app_os: 2,
        name: username.clone(),
        pwd: password,
    };

    // 3. 调用远端 API（不持有锁）
    let response = client.login(&request).await.map_err(|e| e.to_string())?;

    if response.code != 1 {
        return Err(LzuError::LoginFailed(response.message).to_string());
    }

    let data = response
        .data
        .ok_or_else(|| LzuError::LoginFailed("登录响应缺少 data 字段".to_string()).to_string())?;

    let login_token = data
        .login_token
        .ok_or_else(|| LzuError::LoginFailed("登录响应缺少 login_token".to_string()).to_string())?;

    let gateway_token = data.gateway_token.ok_or_else(|| {
        LzuError::LoginFailed("登录响应缺少 gateway_token".to_string()).to_string()
    })?;

    let profile = fetch_profile_summary(&client, &login_token, &gateway_token, &username)
        .await
        .ok()
        .flatten();

    // 4. 重新上锁，更新 session
    let mut auth = lzu_auth.lock().map_err(|e| format!("内部错误: {e}"))?;
    auth.set_session(username, login_token, gateway_token, profile);

    // 持久化到本地 SQLite，重启后自动恢复。
    match db.lock() {
        Ok(conn) => {
            if let Err(e) = auth.persist(&conn) {
                log::error!("LZU 登录持久化失败: {e}");
            }
        }
        Err(e) => log::error!("failed to lock DB for LZU session persist: {e}"),
    }

    log::info!("LZU 登录成功");

    // 登录成功后后台静默自动配置课表（无感导入）。
    // auth 锁持有期间提取所需数据，随后释放锁。
    let auth_status = AuthStatus::from(&auth.session);
    let client_for_auto = auth.client.clone();
    let session_for_auto = auth.session.clone();
    let db_for_auto = db.inner().clone();
    let app_for_auto = app_handle.clone();
    drop(auth);

    if let Some(session) = session_for_auto {
        tauri::async_runtime::spawn(async move {
            auto_import_lzu_after_login(db_for_auto, client_for_auto, session, app_for_auto).await;
        });
    }

    Ok(auth_status)
}

/// 登出 LZU 统一认证。
///
/// 清除本地 session，并尝试调用远端登出接口。
#[tauri::command]
pub async fn lzu_logout(
    db: State<'_, Arc<Mutex<Connection>>>,
    lzu_auth: State<'_, SharedLzuAuth>,
) -> Result<AuthStatus, String> {
    let (client, _, session) = require_lzu_clients(&lzu_auth)?;

    // 调用远端登出（不持有锁）
    match client
        .logout(&session.login_token, &session.gateway_token)
        .await
    {
        Ok(resp) => {
            if resp.code == 1 {
                log::info!("LZU 远端登出成功");
            } else {
                log::warn!(
                    "LZU 远端登出返回异常: code={}, message={}",
                    resp.code,
                    resp.message
                );
            }
        }
        Err(e) => {
            log::warn!("LZU 远端登出请求失败: {e}，本地会话仍将清除");
        }
    }

    // 3. 清除本地 session（内存 + SQLite）
    let mut auth = lzu_auth.lock().map_err(|e| format!("内部错误: {e}"))?;
    auth.session = None;

    match db.lock() {
        Ok(conn) => {
            if let Err(e) = auth.persist(&conn) {
                log::error!("LZU 登出持久化清除失败: {e}");
            }
        }
        Err(e) => log::error!("failed to lock DB for LZU session clear: {e}"),
    }

    log::info!("LZU 本地会话已清除");
    Ok(AuthStatus::from(&auth.session))
}

/// 获取 LZU 登录状态摘要。
///
/// 返回是否登录和用户名，不包含任何 token。
#[tauri::command]
pub fn lzu_get_auth_status(lzu_auth: State<'_, SharedLzuAuth>) -> Result<AuthStatus, String> {
    let auth = lzu_auth.lock().map_err(|e| format!("内部错误: {e}"))?;
    Ok(AuthStatus::from(&auth.session))
}

/// 刷新当前 LZU 登录账号的低敏身份摘要。
/// 会话失效时清除登录态，前端触发重新登录。
#[tauri::command]
pub async fn lzu_refresh_profile(
    db: State<'_, Arc<Mutex<Connection>>>,
    lzu_auth: State<'_, SharedLzuAuth>,
) -> Result<AuthStatus, String> {
    let (client, _, session) = require_lzu_clients(&lzu_auth)?;

    let profile = match fetch_profile_summary(
        &client,
        &session.login_token,
        &session.gateway_token,
        &session.username,
    )
    .await
    {
        Ok(profile) => profile,
        Err(_) => {
            clear_session(&lzu_auth, &db);
            return Err("LZU 会话已过期，请重新登录".to_string());
        }
    };

    let mut auth = lzu_auth.lock().map_err(|e| format!("内部错误: {e}"))?;
    auth.set_profile(profile).map_err(|e| e.to_string())?;
    Ok(AuthStatus::from(&auth.session))
}

/// 刷新 LZU Service Ticket。
///
/// 只更新后端内存会话，不向前端返回 ST 原文。
/// 会话失效时清除登录态，前端触发重新登录。
#[tauri::command]
pub async fn lzu_refresh_st(
    db: State<'_, Arc<Mutex<Connection>>>,
    lzu_auth: State<'_, SharedLzuAuth>,
    service_id: Option<String>,
) -> Result<AuthStatus, String> {
    let (client, _, session) = require_lzu_clients(&lzu_auth)?;

    let response = client
        .get_st(&session.login_token, service_id.as_deref().unwrap_or(""))
        .await
        .map_err(|e| clear_on_err(&lzu_auth, &db, format!("LZU 会话已过期，请重新登录: {e}")))?;

    fail_with_session_clear(&lzu_auth, &db, response.code, response.message)?;

    let st = response.data.ok_or_else(|| {
        LzuError::Api {
            code: response.code,
            message: "getSt 响应缺少 data 字段".to_string(),
        }
        .to_string()
    })?;

    let mut auth = lzu_auth.lock().map_err(|e| format!("内部错误: {e}"))?;
    auth.set_st(st).map_err(|e| e.to_string())?;
    log::info!("LZU ST 已刷新");
    Ok(AuthStatus::from(&auth.session))
}

/// 查询 LZU 校园卡只读余额总览。
#[tauri::command]
pub async fn lzu_get_campus_card_overview(
    db: State<'_, Arc<Mutex<Connection>>>,
    lzu_auth: State<'_, SharedLzuAuth>,
) -> Result<CampusCardOverview, String> {
    let (client, easytong_client, session) = require_lzu_clients(&lzu_auth)?;

    let (easytong_session, account) =
        match refresh_easytong_session(&client, &easytong_client, &session).await {
            Ok(v) => v,
            Err(e) => {
                clear_session(&lzu_auth, &db);
                return Err(e);
            }
        };
    let wallet_response = easytong_client
        .get_wallet_money(&easytong_session)
        .await
        .map_err(|e| clear_on_err(&lzu_auth, &db, e.to_string()))?;
    fail_with_session_clear(&lzu_auth, &db, wallet_response.code, wallet_response.msg.clone())?;

    {
        let mut auth = lzu_auth.lock().map_err(|e| format!("内部错误: {e}"))?;
        auth.set_easytong_session(easytong_session)
            .map_err(|e| e.to_string())?;
    }

    Ok(campus_card_overview(&account, wallet_response))
}

/// 查询 LZU 服务目录低敏摘要。
#[tauri::command]
pub async fn lzu_get_service_directory(
    db: State<'_, Arc<Mutex<Connection>>>,
    lzu_auth: State<'_, SharedLzuAuth>,
) -> Result<LzuServiceDirectory, String> {
    let (client, _, session) = require_lzu_clients(&lzu_auth)?;
    let response = client
        .get_service_directory(&session.login_token)
        .await
        .map_err(|e| clear_on_err(&lzu_auth, &db, e.to_string()))?;
    fail_with_session_clear(&lzu_auth, &db, response.code, response.message.clone())?;

    Ok(sanitize_service_directory(response))
}

/// 在应用内 WebView 窗口中打开 LZU 服务。
///
/// 流程：getSt → 构造 URL（含 PersonID、st、ticket）→ 新 Tauri 窗口加载。
#[tauri::command]
pub async fn lzu_open_service(
    app_handle: tauri::AppHandle,
    db: State<'_, Arc<Mutex<Connection>>>,
    lzu_auth: State<'_, SharedLzuAuth>,
    service_id: String,
    h5_url: String,
) -> Result<(), String> {
    let (client, _, session) = require_lzu_clients(&lzu_auth)?;

    let response = client
        .get_st(&session.login_token, &service_id)
        .await
        .map_err(|e| clear_on_err(&lzu_auth, &db, e.to_string()))?;

    fail_with_session_clear(&lzu_auth, &db, response.code, response.message)?;

    let st = response.data.ok_or_else(|| {
        clear_session(&lzu_auth, &db);
        api_error(response.code, "getSt 响应缺少 data 字段".to_string())
    })?;

    let person_id = &session.username;
    let url = format!("{h5_url}?PersonID={person_id}&st={st}&ticket={st}");

    tauri::WebviewWindowBuilder::new(
        &app_handle,
        "lzu-service",
        tauri::WebviewUrl::External(url.parse().map_err(|e| format!("无效 URL: {e}"))?),
    )
    .title("LZU 服务")
    .inner_size(420.0, 720.0)
    .build()
    .map_err(|e| format!("无法创建服务窗口: {e}"))?;

    Ok(())
}

/// 从 LZU API 拉取课表并导入本地课程表。
///
/// 前端手动触发（"导入课表"按钮），总是全量拉取。
#[tauri::command]
pub async fn import_lzu_courses(
    db: State<'_, Arc<Mutex<Connection>>>,
    lzu_auth: State<'_, SharedLzuAuth>,
) -> Result<LzuCourseImportResult, String> {
    let (client, _, session) = require_lzu_clients(&lzu_auth)?;
    import_lzu_courses_inner(db.inner().clone(), client, session).await
}

/// LZU 课表导入核心逻辑（纯数据参数，不依赖 Tauri State）。
///
/// 同时服务于手动命令 `import_lzu_courses` 与登录后的后台自动导入。
async fn import_lzu_courses_inner(
    db: Arc<Mutex<Connection>>,
    client: Arc<AppServiceClient>,
    session: LzuSession,
) -> Result<LzuCourseImportResult, String> {
    let xlxx = fetch_xlxx(&client, &session).await?;
    let (total_weeks, has_authoritative_total_weeks) = schedule_week_limit(&xlxx)?;

    let mut remote_courses = Vec::new();
    for week_index in 1..=total_weeks {
        let schedule_response = client
            .get_schedule(&session.gateway_token, week_index)
            .await
            .map_err(|e| e.to_string())?;
        if schedule_response.code != 1 {
            if !has_authoritative_total_weeks && !remote_courses.is_empty() {
                log::warn!(
                    "LZU 第 {week_index} 周课表返回异常，fallback 模式停止继续拉取: code={}, message={}",
                    schedule_response.code,
                    schedule_response.message
                );
                break;
            }
            return Err(api_error(schedule_response.code, schedule_response.message));
        }
        if let Some(mut courses) = schedule_response.data {
            remote_courses.append(&mut courses);
        }
    }

    let parsed = remote_courses.len();
    let mut failed = 0usize;
    let mut courses = Vec::new();
    for course in remote_courses {
        match crate::lzu::mapper::map_course(&course, &xlxx) {
            Ok(mapped) => courses.push(mapped),
            Err(err) => {
                failed += 1;
                log::warn!("跳过一条无法映射的 LZU 课程: {err}");
            }
        }
    }

    // 落库段（批量导入 + 学期上下文持久化）移入 spawn_blocking，避免阻塞 async worker。
    let db_for_blocking = db.clone();

    let imported: usize;
    let skipped: usize;
    let message: String;

    if courses.is_empty() {
        let xlxx = xlxx.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let conn = db_for_blocking
                .lock()
                .map_err(|e| format!("内部错误: {e}"))?;
            persist_lzu_semester_context(&conn, &xlxx, None, total_weeks)?;
            Ok::<_, String>(())
        })
        .await
        .map_err(|e| format!("spawn_blocking 连接池已关闭: {e}"))??;

        imported = 0;
        skipped = 0;
        message = format!("已拉取 {parsed} 条 LZU 课程，但没有可导入课程，失败 {failed} 条。");
    } else {
        let semester = courses[0].semester.clone();
        let xlxx = xlxx.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            let conn = db_for_blocking
                .lock()
                .map_err(|e| format!("内部错误: {e}"))?;
            let result = crate::commands::courses::import_new_courses(&conn, &courses, &semester)?;
            persist_lzu_semester_context(&conn, &xlxx, Some(&semester), total_weeks)?;
            Ok::<_, String>(result)
        })
        .await
        .map_err(|e| format!("spawn_blocking 连接池已关闭: {e}"))??;

        imported = result.imported;
        skipped = result.skipped;
        message = format!(
            "已拉取 {parsed} 条 LZU 课程，导入 {imported} 条，跳过 {skipped} 条重复记录，失败 {failed} 条。",
        );
    }

    Ok(LzuCourseImportResult {
        parsed,
        imported,
        skipped,
        failed,
        message,
    })
}

/// 登录成功后后台静默自动配置课表（无感导入）。
///
/// 幂等短路：先拉取远程学期信息，与本地 `semester_context(source='lzu')`
/// 比较 `term_label` + `start_date`，若该学期已导入则跳过全量拉取。
/// 导入完成后通过 Tauri 事件 `lzu-auto-import` 广播结果供前端刷新。
async fn auto_import_lzu_after_login(
    db: Arc<Mutex<Connection>>,
    client: Arc<AppServiceClient>,
    session: LzuSession,
    app_handle: tauri::AppHandle,
) {
    let result = async {
        let xlxx = fetch_xlxx(&client, &session).await?;

        // 幂等判断：远程学期与已导入学期一致则跳过全量拉取。
        // 学期上下文映射失败时降级为全量导入（与手动路径行为一致），
        // 而非抛出错误终止自动导入。
        let already_imported;
        let idempotent_term_label: Option<String>;
        match crate::lzu::mapper::map_semester_context(&xlxx, None) {
            Ok(semester_req) => {
                idempotent_term_label = Some(semester_req.term_label.clone());
                already_imported = match db.lock().map_err(|e| format!("内部错误: {e}")) {
                    Ok(conn) => {
                        match crate::db::semester::get_latest_semester_context_by_source(
                            &conn,
                            crate::db::semester::LZU_SEMESTER_CONTEXT_SOURCE,
                        )
                        .map_err(|e| e.to_string())
                        {
                            Ok(Some(context)) => {
                                context.term_label == semester_req.term_label
                                    && context.start_date == semester_req.start_date
                            }
                            _ => false,
                        }
                    }
                    Err(e) => {
                        log::warn!("LZU 自动导入幂等检查锁获取失败: {e}");
                        false
                    }
                };
            }
            Err(err) => {
                log::warn!("LZU 学期上下文映射失败（降级为全量导入）: {err}");
                already_imported = false;
                idempotent_term_label = None;
            }
        };

        if already_imported {
            let term_label = idempotent_term_label.unwrap_or_default();
            log::info!(
                "LZU 登录后自动导入跳过：学期 {} 已导入（idempotent）",
                term_label
            );
            return Ok(LzuCourseImportResult {
                parsed: 0,
                imported: 0,
                skipped: 0,
                failed: 0,
                message: format!("学期 {} 已导入，无需重复配置。", term_label),
            });
        }

        import_lzu_courses_inner(db, client, session).await
    }
    .await;

    match &result {
        Ok(import_result) => {
            log::info!("LZU 登录后自动配置课表完成: {}", import_result.message);
            let _ = app_handle.emit("lzu-auto-import", import_result);
        }
        Err(err) => {
            log::warn!("LZU 登录后自动配置课表失败: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn xlxx_with_weeks(zzx: Option<&str>, dqrqszzc: Option<&str>) -> XlxxData {
        serde_json::from_value(json!({
            "dqrqszzc": dqrqszzc,
            "zzx": zzx,
            "ksrq": "2026-02-24",
            "xqm": "1",
            "xn": "2026"
        }))
        .expect("valid xlxx fixture")
    }

    #[test]
    fn test_schedule_week_limit_uses_zzx_when_available() {
        assert_eq!(
            schedule_week_limit(&xlxx_with_weeks(Some("16"), Some("3"))),
            Ok((16, true))
        );
    }

    #[test]
    fn test_schedule_week_limit_falls_back_without_zzx() {
        assert_eq!(
            schedule_week_limit(&xlxx_with_weeks(None, Some("3"))),
            Ok((FALLBACK_TOTAL_WEEKS, false))
        );
    }

    #[test]
    fn test_schedule_week_limit_keeps_large_current_week() {
        assert_eq!(
            schedule_week_limit(&xlxx_with_weeks(None, Some("26"))),
            Ok((26, false))
        );
    }

    #[test]
    fn test_schedule_week_limit_rejects_invalid_zzx() {
        assert!(schedule_week_limit(&xlxx_with_weeks(Some("0"), None)).is_err());
        assert!(schedule_week_limit(&xlxx_with_weeks(Some("bad"), None)).is_err());
    }
}
