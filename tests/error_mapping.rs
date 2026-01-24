use rmcp::model::{ErrorCode, ErrorData, IntoContents};
use serde_json::json;
use wtp_mcp_rs::errors::WtpMcpError;

#[test]
fn command_failed_maps_to_error_data_with_details() {
    let data: ErrorData = WtpMcpError::CommandFailed {
        exit_code: 2,
        message: "boom".to_string(),
        stderr: "nope".to_string(),
    }
    .into();

    assert_eq!(data.code, ErrorCode::INTERNAL_ERROR);
    assert!(data.message.contains("boom"));
    let payload = data.data.expect("expected error data");
    assert_eq!(payload["exit_code"], json!(2));
    assert_eq!(payload["stderr"], json!("nope"));
}

#[test]
fn parse_error_maps_to_parse_error_code_with_raw_output() {
    let data: ErrorData = WtpMcpError::ParseError {
        message: "bad".to_string(),
        raw_output: "raw output".to_string(),
    }
    .into();

    assert_eq!(data.code, ErrorCode::PARSE_ERROR);
    let payload = data.data.expect("expected error data");
    assert_eq!(payload["raw_output"], json!("raw output"));
}

#[test]
fn policy_violation_maps_to_invalid_request() {
    let data: ErrorData = WtpMcpError::PolicyViolation {
        message: "nope".to_string(),
    }
    .into();

    assert_eq!(data.code, ErrorCode::INVALID_REQUEST);
    assert!(data.data.is_none());
}

#[test]
fn config_error_maps_to_invalid_params() {
    let data: ErrorData = WtpMcpError::ConfigError {
        message: "bad config".to_string(),
    }
    .into();

    assert_eq!(data.code, ErrorCode::INVALID_PARAMS);
    assert!(data.data.is_none());
}

#[test]
fn into_contents_returns_single_text_entry() {
    let contents = WtpMcpError::PolicyViolation {
        message: "nope".to_string(),
    }
    .into_contents();

    assert_eq!(contents.len(), 1);
}
