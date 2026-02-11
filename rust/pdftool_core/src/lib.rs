// SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[wasm_bindgen]
pub fn detect_blocks(pdf_data: &[u8]) -> Result<JsValue, JsValue> {
    if pdf_data.is_empty() {
        return Err(JsValue::from_str("empty PDF payload"));
    }

    // Placeholder deterministic output to validate extension wiring.
    let blocks = vec![
        Block {
            label: "Full Name".to_owned(),
            x: 100.0,
            y: 140.0,
            width: 160.0,
            height: 20.0,
        },
        Block {
            label: "Date of Birth".to_owned(),
            x: 100.0,
            y: 170.0,
            width: 160.0,
            height: 20.0,
        },
    ];

    serde_wasm_bindgen::to_value(&blocks)
        .map_err(|err| JsValue::from_str(&format!("serialization error: {err}")))
}

#[wasm_bindgen]
pub fn fill_blocks(
    pdf_data: &[u8],
    _blocks: JsValue,
    _fields: JsValue,
) -> Result<js_sys::Uint8Array, JsValue> {
    if pdf_data.is_empty() {
        return Err(JsValue::from_str("empty PDF payload"));
    }

    // Placeholder passthrough implementation.
    Ok(js_sys::Uint8Array::from(pdf_data))
}
