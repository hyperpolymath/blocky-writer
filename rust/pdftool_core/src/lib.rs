// SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest

use lopdf::{Dictionary, Document, Object};
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

fn resolve_object(doc: &Document, obj: &Object) -> Result<Object, lopdf::Error> {
    match obj {
        Object::Reference(id) => doc.get_object(*id).map(Clone::clone),
        _ => Ok(obj.clone()),
    }
}

fn object_to_number(obj: &Object) -> Option<f32> {
    match obj {
        Object::Integer(v) => Some(*v as f32),
        Object::Real(v) => Some(*v as f32),
        _ => None,
    }
}

fn rect_from_object(obj: &Object) -> Option<(f32, f32, f32, f32)> {
    let Object::Array(values) = obj else {
        return None;
    };

    if values.len() != 4 {
        return None;
    }

    let llx = object_to_number(&values[0])?;
    let lly = object_to_number(&values[1])?;
    let urx = object_to_number(&values[2])?;
    let ury = object_to_number(&values[3])?;

    let x = llx.min(urx);
    let y = lly.min(ury);
    let width = (urx - llx).abs();
    let height = (ury - lly).abs();
    Some((x, y, width, height))
}

fn object_to_text(obj: &Object) -> Option<String> {
    let raw = match obj {
        Object::String(bytes, _) => bytes.as_slice(),
        Object::Name(bytes) => bytes.as_slice(),
        _ => return None,
    };

    let text = String::from_utf8_lossy(raw)
        .trim_matches(char::from(0))
        .trim()
        .to_owned();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn dict_text(doc: &Document, dict: &Dictionary, key: &[u8]) -> Option<String> {
    let obj = dict.get(key).ok()?;
    match resolve_object(doc, obj).ok() {
        Some(resolved) => object_to_text(&resolved),
        None => object_to_text(obj),
    }
}

fn widget_label(doc: &Document, widget: &Dictionary, fallback: String) -> String {
    if let Some(label) = dict_text(doc, widget, b"T") {
        return label;
    }

    if let Ok(parent_obj) = widget.get(b"Parent") {
        if let Ok(Object::Dictionary(parent_dict)) = resolve_object(doc, parent_obj) {
            if let Some(label) = dict_text(doc, &parent_dict, b"T") {
                return label;
            }
        }
    }

    fallback
}

#[wasm_bindgen]
pub fn detect_blocks(pdf_data: &[u8]) -> Result<JsValue, JsValue> {
    if pdf_data.is_empty() {
        return Err(JsValue::from_str("empty PDF payload"));
    }

    let doc =
        Document::load_mem(pdf_data).map_err(|err| JsValue::from_str(&format!("invalid PDF: {err}")))?;

    let mut blocks = Vec::<Block>::new();
    for (page_number, page_id) in doc.get_pages() {
        let page_obj = doc
            .get_object(page_id)
            .map_err(|err| JsValue::from_str(&format!("failed to read page {page_number}: {err}")))?;
        let page_dict = page_obj
            .as_dict()
            .map_err(|err| JsValue::from_str(&format!("page {page_number} is not a dictionary: {err}")))?;

        let annots_obj = match page_dict.get(b"Annots") {
            Ok(obj) => obj,
            Err(_) => continue,
        };

        let annots = match resolve_object(&doc, annots_obj) {
            Ok(Object::Array(arr)) => arr,
            Ok(_) | Err(_) => continue,
        };

        for (annot_index, annot_ref) in annots.iter().enumerate() {
            let annot_obj = match resolve_object(&doc, annot_ref) {
                Ok(obj) => obj,
                Err(_) => continue,
            };
            let widget = match annot_obj {
                Object::Dictionary(dict) => dict,
                _ => continue,
            };

            let is_widget = matches!(
                widget.get(b"Subtype"),
                Ok(Object::Name(name)) if name.as_slice() == b"Widget"
            );
            if !is_widget {
                continue;
            }

            let rect = match widget.get(b"Rect").ok().and_then(rect_from_object) {
                Some(rect) => rect,
                None => continue,
            };

            let fallback = format!("field_{}_{}", page_number, annot_index + 1);
            let label = widget_label(&doc, &widget, fallback);

            blocks.push(Block {
                label,
                x: rect.0,
                y: rect.1,
                width: rect.2,
                height: rect.3,
            });
        }
    }

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
