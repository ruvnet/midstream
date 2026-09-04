//! wasm-bindgen exports (feature `wasm`). Results cross the boundary as JSON strings
//! so the JavaScript side needs no generated types.

use wasm_bindgen::prelude::*;

/// `detect` returning the [`crate::Detection`] as a JSON string.
#[wasm_bindgen(js_name = detectJson)]
pub fn detect_json(text: &str) -> String {
    serde_json::to_string(&crate::detect(text)).unwrap_or_else(|_| "{}".to_string())
}

/// See [`crate::is_safe`].
#[wasm_bindgen(js_name = isSafe)]
pub fn is_safe(text: &str) -> bool {
    crate::is_safe(text)
}

/// See [`crate::sanitize`].
#[wasm_bindgen(js_name = sanitize)]
pub fn sanitize(text: &str) -> String {
    crate::sanitize(text)
}

/// See [`crate::normalize`].
#[wasm_bindgen(js_name = normalize)]
pub fn normalize(text: &str) -> String {
    crate::normalize(text)
}

/// Number of bundled threat patterns.
#[wasm_bindgen(js_name = patternCount)]
pub fn pattern_count() -> usize {
    crate::pattern_count()
}
