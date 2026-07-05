use rusqlite::{params, Connection, Error, Result};

use super::models::{UpdateWidgetConfigRequest, WidgetConfig};

const WIDGET_MIN_OPACITY: f64 = 0.6;
const WIDGET_MAX_OPACITY: f64 = 1.0;
const WIDGET_MIN_WIDTH: i64 = 220;
const WIDGET_MAX_WIDTH: i64 = 520;
const WIDGET_MIN_HEIGHT: i64 = 140;
const WIDGET_MAX_HEIGHT: i64 = 420;

pub fn default_widget_size(mode: &str) -> Result<(i64, i64)> {
    match mode {
        "small" => Ok((260, 140)),
        "medium" => Ok((320, 220)),
        "large" => Ok((420, 320)),
        _ => Err(invalid_widget_config("invalid widget mode")),
    }
}

pub fn get_or_create_widget_config(conn: &Connection) -> Result<WidgetConfig> {
    let result = select_widget_config(conn);

    match result {
        Ok(config) => Ok(config),
        Err(Error::QueryReturnedNoRows) => {
            let now = super::chrono_now();
            conn.execute(
                "INSERT INTO widget_config
                    (id, enabled, mode, opacity, always_on_top, locked, width, height, created_at, updated_at)
                 VALUES
                    (1, 0, 'medium', 0.92, 1, 0, 320, 220, ?1, ?1)",
                params![now],
            )?;
            select_widget_config(conn)
        }
        Err(e) => Err(e),
    }
}

pub fn update_widget_config(
    conn: &Connection,
    req: &UpdateWidgetConfigRequest,
) -> Result<WidgetConfig> {
    let current = get_or_create_widget_config(conn)?;
    let next = merge_widget_config(&current, req)?;

    conn.execute(
        "UPDATE widget_config
         SET enabled = ?1,
             mode = ?2,
             opacity = ?3,
             always_on_top = ?4,
             locked = ?5,
             x = ?6,
             y = ?7,
             width = ?8,
             height = ?9,
             updated_at = ?10
         WHERE id = 1",
        params![
            next.enabled as i64,
            next.mode,
            next.opacity,
            next.always_on_top as i64,
            next.locked as i64,
            next.x,
            next.y,
            next.width,
            next.height,
            next.updated_at,
        ],
    )?;

    select_widget_config(conn)
}

pub fn merge_widget_config(
    current: &WidgetConfig,
    req: &UpdateWidgetConfigRequest,
) -> Result<WidgetConfig> {
    let mode = req.mode.clone().unwrap_or_else(|| current.mode.clone());
    let mode_changed = mode != current.mode;
    let default_size = default_widget_size(&mode)?;
    let next = WidgetConfig {
        id: 1,
        enabled: req.enabled.unwrap_or(current.enabled),
        mode,
        opacity: req.opacity.unwrap_or(current.opacity),
        always_on_top: req.always_on_top.unwrap_or(current.always_on_top),
        locked: req.locked.unwrap_or(current.locked),
        x: req.x.or(current.x),
        y: req.y.or(current.y),
        width: req.width.unwrap_or(if mode_changed {
            default_size.0
        } else {
            current.width
        }),
        height: req.height.unwrap_or(if mode_changed {
            default_size.1
        } else {
            current.height
        }),
        created_at: current.created_at.clone(),
        updated_at: super::chrono_now(),
    };

    validate_widget_config(&next)?;
    Ok(next)
}

fn select_widget_config(conn: &Connection) -> Result<WidgetConfig> {
    conn.query_row(
        "SELECT id, enabled, mode, opacity, always_on_top, locked, x, y, width, height, created_at, updated_at
         FROM widget_config WHERE id = 1",
        [],
        |row| {
            Ok(WidgetConfig {
                id: row.get(0)?,
                enabled: row.get::<_, i64>(1)? != 0,
                mode: row.get(2)?,
                opacity: row.get(3)?,
                always_on_top: row.get::<_, i64>(4)? != 0,
                locked: row.get::<_, i64>(5)? != 0,
                x: row.get(6)?,
                y: row.get(7)?,
                width: row.get(8)?,
                height: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        },
    )
}

fn validate_widget_config(config: &WidgetConfig) -> Result<()> {
    default_widget_size(&config.mode)?;
    if !(WIDGET_MIN_OPACITY..=WIDGET_MAX_OPACITY).contains(&config.opacity) {
        return Err(invalid_widget_config(
            "widget opacity must be between 0.6 and 1.0",
        ));
    }
    if !(WIDGET_MIN_WIDTH..=WIDGET_MAX_WIDTH).contains(&config.width) {
        return Err(invalid_widget_config("widget width is out of range"));
    }
    if !(WIDGET_MIN_HEIGHT..=WIDGET_MAX_HEIGHT).contains(&config.height) {
        return Err(invalid_widget_config("widget height is out of range"));
    }
    Ok(())
}

fn invalid_widget_config(message: &str) -> Error {
    Error::InvalidParameterName(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");
        migrations::run_migrations(&conn).expect("Migrations failed");
        conn
    }

    #[test]
    fn test_get_or_create_widget_config_returns_defaults() {
        let conn = setup_db();

        let config = get_or_create_widget_config(&conn).expect("Failed to get widget config");

        assert_eq!(config.id, 1);
        assert!(!config.enabled);
        assert_eq!(config.mode, "medium");
        assert_eq!(config.opacity, 0.92);
        assert!(config.always_on_top);
        assert!(!config.locked);
        assert_eq!((config.width, config.height), (320, 220));
    }

    #[test]
    fn test_update_widget_config_persists_fields() {
        let conn = setup_db();
        let req = UpdateWidgetConfigRequest {
            enabled: Some(true),
            mode: Some("large".to_string()),
            opacity: Some(0.8),
            always_on_top: Some(false),
            locked: Some(true),
            x: Some(120),
            y: Some(80),
            ..Default::default()
        };

        let config = update_widget_config(&conn, &req).expect("Failed to update widget config");

        assert!(config.enabled);
        assert_eq!(config.mode, "large");
        assert_eq!(config.opacity, 0.8);
        assert!(!config.always_on_top);
        assert!(config.locked);
        assert_eq!(config.x, Some(120));
        assert_eq!(config.y, Some(80));
        assert_eq!((config.width, config.height), (420, 320));
    }

    #[test]
    fn test_update_widget_config_rejects_invalid_mode() {
        let conn = setup_db();
        let req = UpdateWidgetConfigRequest {
            mode: Some("wide".to_string()),
            ..Default::default()
        };

        let result = update_widget_config(&conn, &req);

        assert!(result.is_err(), "Invalid mode should be rejected");
    }

    #[test]
    fn test_update_widget_config_rejects_invalid_opacity() {
        let conn = setup_db();
        let req = UpdateWidgetConfigRequest {
            opacity: Some(0.4),
            ..Default::default()
        };

        let result = update_widget_config(&conn, &req);

        assert!(result.is_err(), "Invalid opacity should be rejected");
    }

    #[test]
    fn test_widget_config_constraints_reject_invalid_sql_values() {
        let conn = setup_db();

        let result = conn.execute(
            "UPDATE widget_config SET mode = 'wide', opacity = 1.2 WHERE id = 1",
            [],
        );

        assert!(
            result.is_err(),
            "CHECK constraints should reject invalid values"
        );
    }
}
