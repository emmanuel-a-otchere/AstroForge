use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReportConfig {
    pub enabled: bool,
    pub endpoint: Option<String>,
    pub include_system_info: bool,
    pub include_stack_trace: bool,
}

impl Default for CrashReportConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: None,
            include_system_info: false,
            include_stack_trace: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub session_id: String,
    pub events: Vec<TelemetryEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub event_type: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

impl TelemetryConfig {
    pub fn new() -> Self {
        Self {
            enabled: false,
            session_id: String::new(),
            events: Vec::new(),
        }
    }

    pub fn record_event(&mut self, event_type: &str, data: serde_json::Value) {
        if !self.enabled {
            return;
        }
        self.events.push(TelemetryEvent {
            event_type: event_type.into(),
            timestamp: now_iso(),
            data,
        });
    }

    pub fn flush(&mut self) -> Vec<TelemetryEvent> {
        std::mem::take(&mut self.events)
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self::new()
    }
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("1970-01-01T00:00:{}Z", secs)
}

pub fn generate_crash_report(error: &str, config: &CrashReportConfig) -> serde_json::Value {
    serde_json::json!({
        "error": error,
        "timestamp": now_iso(),
        "system_info": if config.include_system_info {
            serde_json::json!({
                "platform": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
            })
        } else {
            serde_json::Value::Null
        },
        "stack_trace": if config.include_stack_trace {
            serde_json::json!(error)
        } else {
            serde_json::Value::Null
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_report_config_defaults() {
        let config = CrashReportConfig::default();
        assert!(!config.enabled);
        assert!(config.include_stack_trace);
    }

    #[test]
    fn test_telemetry_disabled_no_events() {
        let mut telemetry = TelemetryConfig::new();
        telemetry.record_event("test", serde_json::json!({}));
        assert!(telemetry.events.is_empty());
    }

    #[test]
    fn test_telemetry_enabled_records() {
        let mut telemetry = TelemetryConfig::new();
        telemetry.enabled = true;
        telemetry.record_event("pipeline_started", serde_json::json!({"stages": 5}));
        assert_eq!(telemetry.events.len(), 1);
        assert_eq!(telemetry.events[0].event_type, "pipeline_started");
    }

    #[test]
    fn test_telemetry_flush() {
        let mut telemetry = TelemetryConfig::new();
        telemetry.enabled = true;
        telemetry.record_event("test", serde_json::json!({}));
        let flushed = telemetry.flush();
        assert_eq!(flushed.len(), 1);
        assert!(telemetry.events.is_empty());
    }

    #[test]
    fn test_generate_crash_report() {
        let config = CrashReportConfig {
            enabled: true,
            endpoint: None,
            include_system_info: true,
            include_stack_trace: true,
        };
        let report = generate_crash_report("panic at line 42", &config);
        assert!(report["error"].as_str().unwrap().contains("panic"));
        assert!(report["system_info"]["platform"].is_string());
    }
}
