//! JWT decoding unit tests.

#[path = "ui_helpers.rs"]
mod ui_helpers;

use ui_helpers::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ═══════════════════════════════════════════════════════════════════════════
//  1 · Pure-logic unit tests (JWT decoding)
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_decode_jwt_valid_token() {
    let token = mock_token("my-user-id");
    let result = app::decode_jwt_payload(&token);
    assert!(result.is_some(), "should parse a valid token");
    assert_eq!(result.unwrap().sub, "my-user-id");
}

#[wasm_bindgen_test]
fn test_decode_jwt_missing_segments() {
    assert!(app::decode_jwt_payload("only.two").is_none());
    assert!(app::decode_jwt_payload("single").is_none());
    assert!(app::decode_jwt_payload("").is_none());
}

#[wasm_bindgen_test]
fn test_decode_jwt_invalid_base64() {
    assert!(app::decode_jwt_payload("a.!!!invalid!!!.c").is_none());
}

#[wasm_bindgen_test]
fn test_decode_jwt_invalid_json() {
    use base64::Engine;
    let not_json = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"this is not json");
    let token = format!("header.{}.sig", not_json);
    assert!(app::decode_jwt_payload(&token).is_none());
}
