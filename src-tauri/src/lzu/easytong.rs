//! EasyTong 一卡通只读 client。
//!
//! EasyTong 使用 HTTP 和 MD5 签名。此模块只实现 Phase A 的只读账户/余额能力，
//! 不包含二维码、订单轮询或 WebView SSO。

use chrono::{FixedOffset, Utc};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::lzu::error::LzuError;

const BASE_URL: &str = "http://app.lzu.edu.cn:8080";
const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 12; SM-S7110 Build/V417IR; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/101.0.4951.61 Mobile Safari/537.36 lzdx_ua JHZF_LZDXAPP";
const EXCHANGE_ET_TOKEN_PATH: &str = "/easytong_app/ExchangeEtToken";
const GET_ACC_INFO_PATH: &str = "/easytong_app/GetAccInfo";
const GET_WALLET_MONEY_PATH: &str = "/easytong_app/GetWalletMoney";
const MD5_KEY_ENV: &str = "KAIROS_LZU_MD5_KEY";

#[derive(Debug, Clone)]
pub struct EasyTongSession {
    pub et_token: String,
    pub acc_num: String,
    pub card_acc_num: Option<String>,
    pub epid: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EasyTongAccountInfo {
    pub code: i64,
    pub msg: String,
    pub card_acc_num: Option<String>,
    pub epid: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EasyTongWalletResponse {
    pub code: i64,
    pub msg: String,
    pub wallets: Vec<CampusWallet>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CampusCardOverview {
    pub account: CampusCardAccount,
    pub wallets: Vec<CampusWallet>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CampusCardAccount {
    pub card_tail: Option<String>,
    pub epid_available: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CampusWallet {
    pub card_name: Option<String>,
    pub unit: Option<String>,
    pub wallet_money: Option<String>,
    pub wallet_name: Option<String>,
    pub is_withdraw: Option<String>,
    pub money_max: Option<String>,
    pub mon_temp: Option<String>,
    pub mon_card: Option<String>,
    pub wallet_num: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EtTokenResponse {
    code: i64,
    msg: String,
    #[serde(rename = "accNum")]
    acc_num: Option<String>,
    token: Option<String>,
}

pub struct EasyTongClient {
    client: Client,
}

impl EasyTongClient {
    pub fn new() -> Result<Self, LzuError> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert("User-Agent", HeaderValue::from_static(USER_AGENT));
        default_headers.insert("Host", HeaderValue::from_static("app.lzu.edu.cn:8080"));
        default_headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );

        let client = Client::builder()
            .default_headers(default_headers)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| LzuError::Network(format!("创建 EasyTong HTTP client 失败: {e}")))?;

        Ok(EasyTongClient { client })
    }

    pub async fn exchange_et_token(&self, st: &str) -> Result<EasyTongSession, LzuError> {
        let time = easytong_time();
        let params = [
            ("Time", time.as_str()),
            ("St", st),
            ("ContentType", "application/json"),
        ];
        let body = self
            .post_form(EXCHANGE_ET_TOKEN_PATH, None, &params, "exchange_et_token")
            .await?;
        let parsed: EtTokenResponse = serde_json::from_str(&body)?;
        if parsed.code != 1 {
            return Err(LzuError::Api {
                code: parsed.code,
                message: parsed.msg,
            });
        }

        let acc_num = parsed
            .acc_num
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| LzuError::Internal("EasyTong 响应缺少 accNum".to_string()))?;
        let et_token = parsed
            .token
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| LzuError::Internal("EasyTong 响应缺少 token".to_string()))?;

        Ok(EasyTongSession {
            et_token,
            acc_num,
            card_acc_num: None,
            epid: None,
        })
    }

    pub async fn get_acc_info(
        &self,
        session: &EasyTongSession,
    ) -> Result<EasyTongAccountInfo, LzuError> {
        let time = easytong_time();
        let sign = sign_values(&[session.acc_num.as_str(), time.as_str()])?;
        let params = [
            ("AccNum", session.acc_num.as_str()),
            ("Time", time.as_str()),
            ("Sign", sign.as_str()),
            ("ContentType", "application/json"),
        ];
        let body = self
            .post_form(
                GET_ACC_INFO_PATH,
                Some(session.et_token.as_str()),
                &params,
                "get_acc_info",
            )
            .await?;
        parse_acc_info_xml(&body)
    }

    pub async fn get_wallet_money(
        &self,
        session: &EasyTongSession,
    ) -> Result<EasyTongWalletResponse, LzuError> {
        let epid = session
            .epid
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| LzuError::Internal("EasyTong 账户信息缺少 EPID".to_string()))?;
        let time = easytong_time();
        let sign = sign_values(&[session.acc_num.as_str(), epid, time.as_str()])?;
        let params = [
            ("AccNum", session.acc_num.as_str()),
            ("EPID", epid),
            ("Time", time.as_str()),
            ("Sign", sign.as_str()),
            ("ContentType", "application/json"),
        ];
        let body = self
            .post_form(
                GET_WALLET_MONEY_PATH,
                Some(session.et_token.as_str()),
                &params,
                "get_wallet_money",
            )
            .await?;
        parse_wallet_money_xml(&body)
    }

    async fn post_form(
        &self,
        path: &str,
        authorization: Option<&str>,
        params: &[(&str, &str)],
        operation: &str,
    ) -> Result<String, LzuError> {
        let mut request = self.client.post(format!("{BASE_URL}{path}")).form(params);
        if let Some(token) = authorization {
            request = request.header("Authorization", token);
        }

        let response = request.send().await.map_err(map_easytong_request_error)?;
        crate::lzu::http::read_response_body(response, "EasyTong", operation).await
    }
}

pub fn campus_card_overview(
    account: &EasyTongAccountInfo,
    wallet_response: EasyTongWalletResponse,
) -> CampusCardOverview {
    CampusCardOverview {
        account: CampusCardAccount {
            card_tail: account.card_acc_num.as_deref().and_then(last_four_chars),
            epid_available: account
                .epid
                .as_deref()
                .is_some_and(|value| !value.is_empty()),
        },
        wallets: wallet_response.wallets,
    }
}

fn easytong_time() -> String {
    let china_offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
    Utc::now()
        .with_timezone(&china_offset)
        .format("%Y%m%d%H%M%S")
        .to_string()
}

fn sign_values(values: &[&str]) -> Result<String, LzuError> {
    let key = crate::lzu::config::read_secret(MD5_KEY_ENV)?;
    Ok(sign_values_with_key(values, &key))
}

fn sign_values_with_key(values: &[&str], key: &str) -> String {
    let mut payload = String::new();
    for value in values {
        payload.push_str(value);
        payload.push('|');
    }
    payload.push_str(key);
    format!("{:x}", md5::compute(payload.as_bytes()))
}

fn map_easytong_request_error(err: reqwest::Error) -> LzuError {
    if err.is_timeout() {
        LzuError::Timeout("EasyTong 请求超时".to_string())
    } else {
        LzuError::Network("EasyTong 请求失败".to_string())
    }
}

fn parse_acc_info_xml(body: &str) -> Result<EasyTongAccountInfo, LzuError> {
    let parsed = parse_easytong_xml(body)?;
    Ok(EasyTongAccountInfo {
        code: parse_code(parsed.code.as_deref())?,
        msg: parsed.msg.unwrap_or_default(),
        card_acc_num: parsed.card_acc_num,
        epid: parsed.epid,
    })
}

fn parse_wallet_money_xml(body: &str) -> Result<EasyTongWalletResponse, LzuError> {
    let parsed = parse_easytong_xml(body)?;
    Ok(EasyTongWalletResponse {
        code: parse_code(parsed.code.as_deref())?,
        msg: parsed.msg.unwrap_or_default(),
        wallets: parsed.wallets,
    })
}

#[derive(Default)]
struct EasyTongXmlFields {
    code: Option<String>,
    msg: Option<String>,
    card_acc_num: Option<String>,
    epid: Option<String>,
    wallets: Vec<CampusWallet>,
}

fn parse_easytong_xml(body: &str) -> Result<EasyTongXmlFields, LzuError> {
    let mut reader = Reader::from_str(body);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_tag: Option<String> = None;
    let mut current_wallet: Option<CampusWallet> = None;
    let mut parsed = EasyTongXmlFields::default();
    let mut root_seen = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).to_string();
                if !root_seen {
                    ensure_easytong_root(&tag)?;
                    root_seen = true;
                } else if tag == "Table" {
                    current_wallet = Some(CampusWallet::default());
                } else {
                    current_tag = Some(tag);
                }
            }
            Ok(Event::Empty(event)) => {
                if !root_seen {
                    let tag = String::from_utf8_lossy(event.name().as_ref()).to_string();
                    ensure_easytong_root(&tag)?;
                    root_seen = true;
                }
            }
            Ok(Event::Text(event)) => {
                let value = String::from_utf8_lossy(event.as_ref()).trim().to_string();
                if !value.is_empty() {
                    assign_xml_value(&mut parsed, current_wallet.as_mut(), &current_tag, value);
                }
            }
            Ok(Event::End(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).to_string();
                if tag == "Table" {
                    if let Some(wallet) = current_wallet.take() {
                        parsed.wallets.push(wallet);
                    }
                }
                current_tag = None;
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(LzuError::Json(format!("EasyTong XML 解析失败: {err}"))),
            _ => {}
        }
        buf.clear();
    }

    if !root_seen {
        return Err(LzuError::Json("EasyTong XML 缺少根节点".to_string()));
    }

    Ok(parsed)
}

fn ensure_easytong_root(tag: &str) -> Result<(), LzuError> {
    if tag == "EasyTong" {
        Ok(())
    } else {
        Err(LzuError::Json(format!("EasyTong XML 根节点异常: {tag}")))
    }
}

fn assign_xml_value(
    parsed: &mut EasyTongXmlFields,
    wallet: Option<&mut CampusWallet>,
    tag: &Option<String>,
    value: String,
) {
    let Some(tag) = tag.as_deref() else {
        return;
    };

    if let Some(wallet) = wallet {
        assign_wallet_value(wallet, tag, value);
        return;
    }

    match tag {
        "Code" => parsed.code = Some(value),
        "Msg" => parsed.msg = Some(value),
        "CardAccNum" => parsed.card_acc_num = Some(value),
        "EPID" => parsed.epid = Some(value),
        _ => {}
    }
}

fn assign_wallet_value(wallet: &mut CampusWallet, tag: &str, value: String) {
    match tag {
        "CardName" => wallet.card_name = Some(value),
        "Unit" => wallet.unit = Some(value),
        "WalletMoney" => wallet.wallet_money = Some(value),
        "WalletName" => wallet.wallet_name = Some(value),
        "IsWithdraw" => wallet.is_withdraw = Some(value),
        "MoneyMax" => wallet.money_max = Some(value),
        "MonTemp" => wallet.mon_temp = Some(value),
        "MonCard" => wallet.mon_card = Some(value),
        "WalletNum" => wallet.wallet_num = Some(value),
        _ => {}
    }
}

fn parse_code(value: Option<&str>) -> Result<i64, LzuError> {
    value
        .ok_or_else(|| LzuError::Json("EasyTong XML 缺少 Code".to_string()))?
        .parse::<i64>()
        .map_err(|_| LzuError::Json("EasyTong XML Code 无法解析".to_string()))
}

fn last_four_chars(value: &str) -> Option<String> {
    let chars: Vec<char> = value.trim().chars().collect();
    if chars.len() < 4 {
        return None;
    }
    Some(chars[chars.len() - 4..].iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_values_uses_insertion_order_with_trailing_separator() {
        assert_eq!(
            sign_values_with_key(&["A", "B"], "KEY"),
            "9232e09adf8ff68955d5aa1a7100f95a"
        );
        assert_eq!(
            sign_values_with_key(&["acc", "epid", "20250101120000"], "secret"),
            "f63d0cc33083ef8875a3bc2060700581"
        );
    }

    #[test]
    fn test_parse_acc_info_xml_allows_missing_optional_fields() {
        let response = parse_acc_info_xml(
            "<EasyTong><Code>1</Code><Msg>成功</Msg><CardAccNum>12345678</CardAccNum></EasyTong>",
        )
        .expect("parse xml");

        assert_eq!(response.code, 1);
        assert_eq!(response.msg, "成功");
        assert_eq!(response.card_acc_num.as_deref(), Some("12345678"));
        assert_eq!(response.epid, None);
    }

    #[test]
    fn test_parse_wallet_money_xml() {
        let response = parse_wallet_money_xml(
            "<EasyTong><Code>1</Code><Msg>成功</Msg><Table><CardName>校园卡</CardName><WalletMoney>12.34</WalletMoney><WalletName>主钱包</WalletName><WalletNum>01</WalletNum></Table><Table><WalletMoney>5.00</WalletMoney></Table></EasyTong>",
        )
        .expect("parse wallet xml");

        assert_eq!(response.code, 1);
        assert_eq!(response.wallets.len(), 2);
        assert_eq!(response.wallets[0].wallet_name.as_deref(), Some("主钱包"));
        assert_eq!(response.wallets[0].wallet_money.as_deref(), Some("12.34"));
        assert_eq!(response.wallets[1].wallet_money.as_deref(), Some("5.00"));
    }

    #[test]
    fn test_parse_wallet_money_rejects_missing_code() {
        assert!(parse_wallet_money_xml("<EasyTong><Msg>失败</Msg></EasyTong>").is_err());
    }

    #[test]
    fn test_parse_wallet_money_rejects_unexpected_root() {
        assert!(parse_wallet_money_xml("<Other><Code>1</Code></Other>").is_err());
    }

    #[test]
    fn test_campus_card_overview_masks_card_number() {
        let account = EasyTongAccountInfo {
            code: 1,
            msg: "成功".to_string(),
            card_acc_num: Some("12345678".to_string()),
            epid: Some("epid".to_string()),
        };
        let wallet_response = EasyTongWalletResponse {
            code: 1,
            msg: "成功".to_string(),
            wallets: vec![CampusWallet {
                wallet_money: Some("12.34".to_string()),
                ..CampusWallet::default()
            }],
        };

        let overview = campus_card_overview(&account, wallet_response);

        assert_eq!(overview.account.card_tail.as_deref(), Some("5678"));
        assert!(overview.account.epid_available);
        assert_eq!(overview.wallets.len(), 1);
    }
}
