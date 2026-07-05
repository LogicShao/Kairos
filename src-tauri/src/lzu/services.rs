//! LZU 服务目录低敏模型。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceDirectoryApiResponse {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Vec<ServiceCategoryData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceCategoryData {
    #[serde(default)]
    pub service_type_id: Option<String>,
    #[serde(default)]
    pub service_type_name: Option<String>,
    #[serde(default)]
    pub service_type_icon_url: Option<String>,
    #[serde(default)]
    pub service_type_sort: Option<i64>,
    #[serde(default)]
    pub service_infos: Vec<ServiceInfoData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceInfoData {
    #[serde(default)]
    pub service_info_id: Option<String>,
    #[serde(default)]
    pub service_name: Option<String>,
    #[serde(default)]
    pub app_icon_url: Option<String>,
    #[serde(default)]
    pub pc_icon_url: Option<String>,
    #[serde(default)]
    pub service_type_str: Option<String>,
    #[serde(default)]
    pub service_sort: Option<i64>,
    #[serde(default)]
    pub is_login: Option<i64>,
    #[serde(default)]
    pub is_new: Option<i64>,
    #[serde(default)]
    pub is_top: Option<i64>,
    #[serde(default)]
    pub is_hot: Option<i64>,
    #[serde(default)]
    pub introduce: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LzuServiceDirectory {
    pub categories: Vec<LzuServiceCategory>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LzuServiceCategory {
    pub id: Option<String>,
    pub name: String,
    pub icon_url: Option<String>,
    pub sort: i64,
    pub services: Vec<LzuServiceItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LzuServiceItem {
    pub id: Option<String>,
    pub name: String,
    pub icon_url: Option<String>,
    pub category_name: Option<String>,
    pub introduce: Option<String>,
    pub requires_login: bool,
    pub is_new: bool,
    pub is_top: bool,
    pub is_hot: bool,
    pub sort: i64,
}

pub fn sanitize_service_directory(response: ServiceDirectoryApiResponse) -> LzuServiceDirectory {
    let mut categories: Vec<LzuServiceCategory> = response
        .data
        .into_iter()
        .map(|category| {
            let mut services: Vec<LzuServiceItem> = category
                .service_infos
                .into_iter()
                .map(|service| LzuServiceItem {
                    id: normalized_optional(service.service_info_id),
                    name: normalized_optional(service.service_name)
                        .unwrap_or_else(|| "未命名服务".to_string()),
                    icon_url: normalized_optional(service.app_icon_url)
                        .or_else(|| normalized_optional(service.pc_icon_url)),
                    category_name: normalized_optional(service.service_type_str),
                    introduce: normalized_optional(service.introduce),
                    requires_login: flag(service.is_login),
                    is_new: flag(service.is_new),
                    is_top: flag(service.is_top),
                    is_hot: flag(service.is_hot),
                    sort: service.service_sort.unwrap_or(0),
                })
                .collect();
            services
                .sort_by(|left, right| left.sort.cmp(&right.sort).then(left.name.cmp(&right.name)));

            LzuServiceCategory {
                id: normalized_optional(category.service_type_id),
                name: normalized_optional(category.service_type_name)
                    .unwrap_or_else(|| "未分类".to_string()),
                icon_url: normalized_optional(category.service_type_icon_url),
                sort: category.service_type_sort.unwrap_or(0),
                services,
            }
        })
        .filter(|category| !category.services.is_empty())
        .collect();
    categories.sort_by(|left, right| left.sort.cmp(&right.sort).then(left.name.cmp(&right.name)));

    LzuServiceDirectory { categories }
}

fn normalized_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn flag(value: Option<i64>) -> bool {
    value.unwrap_or(0) != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sanitize_service_directory_keeps_low_sensitive_fields() {
        let response: ServiceDirectoryApiResponse = serde_json::from_value(json!({
            "code": 1,
            "message": "成功",
            "data": [
                {
                    "service_type_id": "type-1",
                    "service_type_name": "办事",
                    "service_type_icon_url": "https://example.test/type.png",
                    "service_type_sort": 2,
                    "service_infos": [
                        {
                            "service_info_id": "svc-1",
                            "service_name": "成绩查询",
                            "app_icon_url": "https://example.test/icon.png",
                            "service_type_str": "教务",
                            "service_sort": 1,
                            "is_login": 1,
                            "is_hot": 1,
                            "introduce": "查询成绩",
                            "h5_service_url": "https://sensitive.example.test",
                            "sign_key": "secret"
                        }
                    ]
                }
            ]
        }))
        .expect("valid fixture");

        let directory = sanitize_service_directory(response);

        assert_eq!(directory.categories.len(), 1);
        assert_eq!(directory.categories[0].name, "办事");
        assert_eq!(directory.categories[0].services[0].name, "成绩查询");
        assert!(directory.categories[0].services[0].requires_login);
        assert!(directory.categories[0].services[0].is_hot);

        let serialized = serde_json::to_string(&directory).expect("serialize directory");
        assert!(!serialized.contains("h5_service_url"));
        assert!(!serialized.contains("sign_key"));
        assert!(!serialized.contains("secret"));
    }

    #[test]
    fn test_sanitize_service_directory_drops_empty_categories() {
        let response = ServiceDirectoryApiResponse {
            code: 1,
            message: "成功".to_string(),
            data: vec![ServiceCategoryData {
                service_type_id: None,
                service_type_name: Some("空分类".to_string()),
                service_type_icon_url: None,
                service_type_sort: Some(1),
                service_infos: Vec::new(),
            }],
        };

        let directory = sanitize_service_directory(response);

        assert!(directory.categories.is_empty());
    }
}
