// SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest

use std::collections::{HashMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId};
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

#[derive(Debug, Serialize)]
struct CoreErrorPayload {
    code: &'static str,
    message: String,
    context: Option<String>,
}

type CoreResult<T> = Result<T, JsValue>;

fn core_error(code: &'static str, message: impl Into<String>) -> JsValue {
    core_error_with_context(code, message, None)
}

fn core_error_with_context(
    code: &'static str,
    message: impl Into<String>,
    context: Option<String>,
) -> JsValue {
    let payload = CoreErrorPayload {
        code,
        message: message.into(),
        context,
    };
    serde_wasm_bindgen::to_value(&payload)
        .unwrap_or_else(|_| JsValue::from_str(&format!("{}: {}", payload.code, payload.message)))
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

fn object_to_name(obj: &Object) -> Option<String> {
    let Object::Name(name) = obj else {
        return None;
    };
    let value = String::from_utf8_lossy(name).trim().to_owned();
    if value.is_empty() {
        None
    } else {
        Some(value)
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

fn object_as_reference(obj: &Object) -> Option<ObjectId> {
    match obj {
        Object::Reference(id) => Some(*id),
        _ => None,
    }
}

fn get_dict<'a>(doc: &'a Document, id: ObjectId, code: &'static str, context: &str) -> CoreResult<&'a Dictionary> {
    let object = doc
        .get_object(id)
        .map_err(|err| core_error_with_context(code, err.to_string(), Some(context.to_owned())))?;
    object
        .as_dict()
        .map_err(|err| core_error_with_context(code, err.to_string(), Some(context.to_owned())))
}

fn get_dict_mut<'a>(
    doc: &'a mut Document,
    id: ObjectId,
    code: &'static str,
    context: &str,
) -> CoreResult<&'a mut Dictionary> {
    let object = doc
        .get_object_mut(id)
        .map_err(|err| core_error_with_context(code, err.to_string(), Some(context.to_owned())))?;
    object
        .as_dict_mut()
        .map_err(|err| core_error_with_context(code, err.to_string(), Some(context.to_owned())))
}

fn is_widget_dict(dict: &Dictionary) -> bool {
    matches!(
        dict.get(b"Subtype"),
        Ok(Object::Name(name)) if name.as_slice() == b"Widget"
    )
}

fn is_widget_object(doc: &Document, id: ObjectId) -> bool {
    matches!(doc.get_object(id), Ok(Object::Dictionary(dict)) if is_widget_dict(dict))
}

fn root_catalog_id(doc: &Document) -> CoreResult<ObjectId> {
    let root = doc
        .trailer
        .get(b"Root")
        .map_err(|err| core_error_with_context("BW_PDF_ROOT_MISSING", err.to_string(), Some("trailer.Root".into())))?;
    object_as_reference(root).ok_or_else(|| {
        core_error_with_context(
            "BW_PDF_ROOT_INVALID",
            "trailer.Root is not an object reference",
            Some("trailer.Root".into()),
        )
    })
}

fn ensure_acroform_object(doc: &mut Document, catalog_id: ObjectId) -> CoreResult<ObjectId> {
    enum FormSource {
        Ref(ObjectId),
        Inline(Dictionary),
    }

    let form_source = {
        let catalog = get_dict(doc, catalog_id, "BW_FORM_CATALOG_INVALID", "catalog dictionary")?;
        let acro_form = catalog.get(b"AcroForm").map_err(|err| {
            core_error_with_context(
                "BW_FORM_MISSING_ACROFORM",
                err.to_string(),
                Some("catalog.AcroForm".into()),
            )
        })?;
        match acro_form {
            Object::Reference(id) => FormSource::Ref(*id),
            Object::Dictionary(dict) => FormSource::Inline(dict.clone()),
            _ => {
                return Err(core_error_with_context(
                    "BW_FORM_ACROFORM_INVALID",
                    "catalog.AcroForm must be a dictionary or reference",
                    Some("catalog.AcroForm".into()),
                ))
            }
        }
    };

    match form_source {
        FormSource::Ref(id) => Ok(id),
        FormSource::Inline(dict) => {
            let form_id = doc.add_object(Object::Dictionary(dict));
            let catalog = get_dict_mut(doc, catalog_id, "BW_FORM_CATALOG_INVALID", "catalog dictionary")?;
            catalog.set(b"AcroForm", Object::Reference(form_id));
            Ok(form_id)
        }
    }
}

fn collect_field_ids(doc: &Document, source: &Object, out: &mut Vec<ObjectId>, seen: &mut HashSet<ObjectId>) {
    match source {
        Object::Reference(id) => {
            if !seen.insert(*id) {
                return;
            }
            out.push(*id);
            if let Ok(Object::Dictionary(dict)) = doc.get_object(*id) {
                if let Ok(kids) = dict.get(b"Kids") {
                    collect_field_ids(doc, kids, out, seen);
                }
            }
        }
        Object::Array(items) => {
            for item in items {
                collect_field_ids(doc, item, out, seen);
            }
        }
        Object::Dictionary(dict) => {
            if let Ok(kids) = dict.get(b"Kids") {
                collect_field_ids(doc, kids, out, seen);
            }
        }
        _ => {}
    }
}

fn field_parent_id(doc: &Document, field_id: ObjectId) -> Option<ObjectId> {
    let dict = match doc.get_object(field_id).ok()?.as_dict() {
        Ok(dict) => dict,
        Err(_) => return None,
    };
    dict.get(b"Parent").ok().and_then(object_as_reference)
}

fn field_partial_name(doc: &Document, field_id: ObjectId) -> Option<String> {
    let dict = doc.get_object(field_id).ok()?.as_dict().ok()?;
    dict_text(doc, dict, b"T")
}

fn field_full_name(doc: &Document, field_id: ObjectId, depth: usize) -> Option<String> {
    if depth > 48 {
        return None;
    }
    let partial = field_partial_name(doc, field_id);
    match (field_parent_id(doc, field_id), partial) {
        (Some(parent_id), Some(name)) => match field_full_name(doc, parent_id, depth + 1) {
            Some(parent_name) => Some(format!("{}.{}", parent_name, name)),
            None => Some(name),
        },
        (Some(parent_id), None) => field_full_name(doc, parent_id, depth + 1),
        (None, Some(name)) => Some(name),
        (None, None) => None,
    }
}

fn field_type(doc: &Document, field_id: ObjectId, depth: usize) -> Option<String> {
    if depth > 48 {
        return None;
    }
    let dict = doc.get_object(field_id).ok()?.as_dict().ok()?;
    if let Ok(ft_obj) = dict.get(b"FT") {
        if let Ok(resolved) = resolve_object(doc, ft_obj) {
            if let Some(name) = object_to_name(&resolved) {
                return Some(name);
            }
        }
        if let Some(name) = object_to_name(ft_obj) {
            return Some(name);
        }
    }

    match field_parent_id(doc, field_id) {
        Some(parent_id) => field_type(doc, parent_id, depth + 1),
        None => None,
    }
}

fn collect_widget_ids_for_field(doc: &Document, source: &Object, out: &mut Vec<ObjectId>, seen: &mut HashSet<ObjectId>) {
    match source {
        Object::Reference(id) => {
            if !seen.insert(*id) {
                return;
            }
            if is_widget_object(doc, *id) {
                out.push(*id);
            }
            if let Ok(Object::Dictionary(dict)) = doc.get_object(*id) {
                if let Ok(kids) = dict.get(b"Kids") {
                    collect_widget_ids_for_field(doc, kids, out, seen);
                }
            }
        }
        Object::Array(items) => {
            for item in items {
                collect_widget_ids_for_field(doc, item, out, seen);
            }
        }
        Object::Dictionary(dict) => {
            if is_widget_dict(dict) {
                // Inline widgets are rare; no stable ObjectId to push.
            }
            if let Ok(kids) = dict.get(b"Kids") {
                collect_widget_ids_for_field(doc, kids, out, seen);
            }
        }
        _ => {}
    }
}

#[derive(Debug)]
struct FieldDescriptor {
    id: ObjectId,
    partial_name: Option<String>,
    full_name: Option<String>,
    field_type: Option<String>,
    widget_ids: Vec<ObjectId>,
}

fn describe_field(doc: &Document, field_id: ObjectId) -> FieldDescriptor {
    let partial_name = field_partial_name(doc, field_id);
    let full_name = field_full_name(doc, field_id, 0);
    let field_type = field_type(doc, field_id, 0);

    let mut widget_ids = Vec::new();
    let mut seen = HashSet::new();
    if is_widget_object(doc, field_id) {
        widget_ids.push(field_id);
        seen.insert(field_id);
    }
    if let Ok(Object::Dictionary(dict)) = doc.get_object(field_id) {
        if let Ok(kids) = dict.get(b"Kids") {
            collect_widget_ids_for_field(doc, kids, &mut widget_ids, &mut seen);
        }
    }

    FieldDescriptor {
        id: field_id,
        partial_name,
        full_name,
        field_type,
        widget_ids,
    }
}

fn field_input_value(descriptor: &FieldDescriptor, fields: &HashMap<String, String>) -> Option<String> {
    if let Some(full_name) = &descriptor.full_name {
        if let Some(value) = fields.get(full_name) {
            return Some(value.clone());
        }
    }
    if let Some(partial_name) = &descriptor.partial_name {
        if let Some(value) = fields.get(partial_name) {
            return Some(value.clone());
        }
    }
    None
}

fn widget_on_state(doc: &Document, widget_id: ObjectId) -> Option<Vec<u8>> {
    let widget = doc.get_object(widget_id).ok()?.as_dict().ok()?;
    let ap = widget.get(b"AP").ok()?;
    let ap_resolved = resolve_object(doc, ap).ok()?;
    let ap_dict = ap_resolved.as_dict().ok()?;
    let normal = ap_dict.get(b"N").ok()?;
    let normal_resolved = resolve_object(doc, normal).ok()?;
    let normal_dict = normal_resolved.as_dict().ok()?;
    for (state_name, _) in normal_dict.iter() {
        if state_name.as_slice() != b"Off" {
            return Some(state_name.clone());
        }
    }
    None
}

fn set_widget_as(doc: &mut Document, widget_id: ObjectId, value: Vec<u8>) -> CoreResult<()> {
    let widget = get_dict_mut(
        doc,
        widget_id,
        "BW_FILL_WIDGET_UPDATE_FAILED",
        &format!("widget {:?}", widget_id),
    )?;
    widget.set(b"AS", Object::Name(value));
    Ok(())
}

fn set_field_text_value(doc: &mut Document, descriptor: &FieldDescriptor, value: &str) -> CoreResult<()> {
    let field = get_dict_mut(
        doc,
        descriptor.id,
        "BW_FILL_FIELD_UPDATE_FAILED",
        &format!("field {:?}", descriptor.id),
    )?;
    field.set(b"V", Object::string_literal(value));
    field.set(b"DV", Object::string_literal(value));
    Ok(())
}

fn is_truthy(value: &str) -> bool {
    matches!(value, "true" | "yes" | "on" | "1" | "checked" | "x")
}

fn is_falsey(value: &str) -> bool {
    matches!(value, "" | "false" | "no" | "off" | "0" | "unchecked")
}

fn set_button_value(doc: &mut Document, descriptor: &FieldDescriptor, raw_value: &str) -> CoreResult<()> {
    let normalized = raw_value.trim().to_ascii_lowercase();

    let widget_states: Vec<(ObjectId, Vec<u8>)> = descriptor
        .widget_ids
        .iter()
        .filter_map(|id| widget_on_state(doc, *id).map(|state| (*id, state)))
        .collect();

    let mut field_value = b"Off".to_vec();
    if descriptor.widget_ids.len() > 1 && !is_truthy(&normalized) && !is_falsey(&normalized) {
        let requested = normalized.as_bytes();
        let selected = widget_states
            .iter()
            .find(|(_, state)| state.eq_ignore_ascii_case(requested))
            .cloned();

        if let Some((selected_widget_id, selected_state)) = selected {
            field_value = selected_state.clone();
            for widget_id in &descriptor.widget_ids {
                if *widget_id == selected_widget_id {
                    set_widget_as(doc, *widget_id, selected_state.clone())?;
                } else {
                    set_widget_as(doc, *widget_id, b"Off".to_vec())?;
                }
            }
        } else {
            return Err(core_error_with_context(
                "BW_FILL_BUTTON_VALUE_INVALID",
                format!("button value '{}' does not match available widget states", raw_value),
                descriptor.full_name.clone().or(descriptor.partial_name.clone()),
            ));
        }
    } else if is_truthy(&normalized) {
        if descriptor.widget_ids.len() > 1 {
            let chosen_widget = descriptor.widget_ids[0];
            for widget_id in &descriptor.widget_ids {
                if *widget_id == chosen_widget {
                    let state = widget_on_state(doc, *widget_id).unwrap_or_else(|| b"Yes".to_vec());
                    field_value = state.clone();
                    set_widget_as(doc, *widget_id, state)?;
                } else {
                    set_widget_as(doc, *widget_id, b"Off".to_vec())?;
                }
            }
        } else {
            field_value = descriptor
                .widget_ids
                .first()
                .and_then(|id| widget_on_state(doc, *id))
                .unwrap_or_else(|| b"Yes".to_vec());
            for widget_id in &descriptor.widget_ids {
                let widget_value = widget_on_state(doc, *widget_id).unwrap_or_else(|| field_value.clone());
                set_widget_as(doc, *widget_id, widget_value)?;
            }
        }
    } else {
        for widget_id in &descriptor.widget_ids {
            set_widget_as(doc, *widget_id, b"Off".to_vec())?;
        }
    }

    let field = get_dict_mut(
        doc,
        descriptor.id,
        "BW_FILL_FIELD_UPDATE_FAILED",
        &format!("field {:?}", descriptor.id),
    )?;
    field.set(b"V", Object::Name(field_value));
    Ok(())
}

fn apply_field_value(doc: &mut Document, descriptor: &FieldDescriptor, value: &str) -> CoreResult<()> {
    let field_type = descriptor
        .field_type
        .clone()
        .unwrap_or_else(|| "Tx".to_string());

    match field_type.as_str() {
        "Tx" | "Ch" => set_field_text_value(doc, descriptor, value),
        "Btn" => set_button_value(doc, descriptor, value),
        other => Err(core_error_with_context(
            "BW_FILL_UNSUPPORTED_FIELD_TYPE",
            format!("unsupported PDF form field type '{}'", other),
            descriptor.full_name.clone().or(descriptor.partial_name.clone()),
        )),
    }
}

#[wasm_bindgen]
pub fn detect_blocks(pdf_data: &[u8]) -> Result<JsValue, JsValue> {
    if pdf_data.is_empty() {
        return Err(core_error("BW_PDF_EMPTY", "empty PDF payload"));
    }

    let doc = Document::load_mem(pdf_data)
        .map_err(|err| core_error_with_context("BW_PDF_INVALID", err.to_string(), Some("Document::load_mem".into())))?;

    let mut blocks = Vec::<Block>::new();
    for (page_number, page_id) in doc.get_pages() {
        let page_obj = doc
            .get_object(page_id)
            .map_err(|err| {
                core_error_with_context(
                    "BW_PDF_PAGE_READ_FAILED",
                    err.to_string(),
                    Some(format!("page {}", page_number)),
                )
            })?;
        let page_dict = page_obj
            .as_dict()
            .map_err(|err| {
                core_error_with_context(
                    "BW_PDF_PAGE_INVALID",
                    err.to_string(),
                    Some(format!("page {}", page_number)),
                )
            })?;

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

            let is_widget = is_widget_dict(&widget);
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
        .map_err(|err| core_error_with_context("BW_SERIALIZATION_ERROR", err.to_string(), Some("detect_blocks".into())))
}

#[wasm_bindgen]
pub fn fill_blocks(
    pdf_data: &[u8],
    blocks: JsValue,
    fields: JsValue,
) -> Result<js_sys::Uint8Array, JsValue> {
    if pdf_data.is_empty() {
        return Err(core_error("BW_PDF_EMPTY", "empty PDF payload"));
    }

    let _requested_blocks: Vec<Block> = serde_wasm_bindgen::from_value(blocks).map_err(|err| {
        core_error_with_context(
            "BW_BLOCKS_PAYLOAD_INVALID",
            err.to_string(),
            Some("fill_blocks blocks argument".into()),
        )
    })?;

    let field_values: HashMap<String, String> = serde_wasm_bindgen::from_value(fields).map_err(|err| {
        core_error_with_context(
            "BW_FIELDS_PAYLOAD_INVALID",
            err.to_string(),
            Some("fill_blocks fields argument".into()),
        )
    })?;

    let mut doc = Document::load_mem(pdf_data)
        .map_err(|err| core_error_with_context("BW_PDF_INVALID", err.to_string(), Some("Document::load_mem".into())))?;

    let catalog_id = root_catalog_id(&doc)?;
    let acroform_id = ensure_acroform_object(&mut doc, catalog_id)?;

    {
        let acroform = get_dict_mut(
            &mut doc,
            acroform_id,
            "BW_FORM_ACROFORM_INVALID",
            "AcroForm dictionary",
        )?;
        acroform.set(b"NeedAppearances", Object::Boolean(true));
    }

    let field_roots = {
        let acroform = get_dict(
            &doc,
            acroform_id,
            "BW_FORM_ACROFORM_INVALID",
            "AcroForm dictionary",
        )?;
        acroform.get(b"Fields").map_err(|err| {
            core_error_with_context("BW_FORM_FIELDS_MISSING", err.to_string(), Some("AcroForm.Fields".into()))
        })?.clone()
    };

    let mut field_ids = Vec::new();
    let mut seen = HashSet::new();
    collect_field_ids(&doc, &field_roots, &mut field_ids, &mut seen);

    if field_ids.is_empty() {
        return Err(core_error(
            "BW_FORM_FIELDS_EMPTY",
            "AcroForm.Fields does not contain fillable fields",
        ));
    }

    let descriptors: Vec<FieldDescriptor> = field_ids
        .iter()
        .map(|id| describe_field(&doc, *id))
        .collect();

    let mut updated_fields = 0usize;
    for descriptor in descriptors {
        let Some(value) = field_input_value(&descriptor, &field_values) else {
            continue;
        };
        apply_field_value(&mut doc, &descriptor, &value)?;
        updated_fields += 1;
    }

    if updated_fields == 0 && !field_values.is_empty() {
        return Err(core_error(
            "BW_FILL_NO_MATCHING_FIELDS",
            "none of the provided input keys matched PDF form field names",
        ));
    }

    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|err| core_error_with_context("BW_FILL_SAVE_FAILED", err.to_string(), Some("Document::save_to".into())))?;

    Ok(js_sys::Uint8Array::from(output.as_slice()))
}
