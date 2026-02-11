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

type CoreResult<T> = Result<T, CoreErrorPayload>;

fn core_error(code: &'static str, message: impl Into<String>) -> CoreErrorPayload {
    core_error_with_context(code, message, None)
}

fn core_error_with_context(
    code: &'static str,
    message: impl Into<String>,
    context: Option<String>,
) -> CoreErrorPayload {
    CoreErrorPayload {
        code,
        message: message.into(),
        context,
    }
}

fn core_error_to_js(payload: CoreErrorPayload) -> JsValue {
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

fn detect_blocks_impl(pdf_data: &[u8]) -> CoreResult<Vec<Block>> {
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

    Ok(blocks)
}

fn fill_blocks_impl(pdf_data: &[u8], field_values: HashMap<String, String>) -> CoreResult<Vec<u8>> {
    if pdf_data.is_empty() {
        return Err(core_error("BW_PDF_EMPTY", "empty PDF payload"));
    }

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

    Ok(output)
}

#[wasm_bindgen]
pub fn detect_blocks(pdf_data: &[u8]) -> Result<JsValue, JsValue> {
    let blocks = detect_blocks_impl(pdf_data).map_err(core_error_to_js)?;
    serde_wasm_bindgen::to_value(&blocks)
        .map_err(|err| core_error_to_js(core_error_with_context("BW_SERIALIZATION_ERROR", err.to_string(), Some("detect_blocks".into()))))
}

#[wasm_bindgen]
pub fn fill_blocks(
    pdf_data: &[u8],
    blocks: JsValue,
    fields: JsValue,
) -> Result<js_sys::Uint8Array, JsValue> {
    let _requested_blocks: Vec<Block> = serde_wasm_bindgen::from_value(blocks).map_err(|err| {
        core_error_to_js(core_error_with_context(
            "BW_BLOCKS_PAYLOAD_INVALID",
            err.to_string(),
            Some("fill_blocks blocks argument".into()),
        ))
    })?;

    let field_values: HashMap<String, String> = serde_wasm_bindgen::from_value(fields).map_err(|err| {
        core_error_to_js(core_error_with_context(
            "BW_FIELDS_PAYLOAD_INVALID",
            err.to_string(),
            Some("fill_blocks fields argument".into()),
        ))
    })?;
    let output = fill_blocks_impl(pdf_data, field_values).map_err(core_error_to_js)?;
    Ok(js_sys::Uint8Array::from(output.as_slice()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Stream};

    fn name(value: &str) -> Object {
        Object::Name(value.as_bytes().to_vec())
    }

    fn rect(llx: i64, lly: i64, urx: i64, ury: i64) -> Object {
        Object::Array(vec![llx.into(), lly.into(), urx.into(), ury.into()])
    }

    fn assert_error_code(result: CoreResult<Vec<u8>>, expected_code: &str) {
        let payload = result.expect_err("expected fill_blocks to fail");
        assert_eq!(payload.code, expected_code);
        assert!(!payload.message.is_empty(), "error message should not be empty");
    }

    fn make_fixture_pdf() -> Vec<u8> {
        let mut doc = Document::with_version("1.7");

        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();
        let catalog_id = doc.new_object_id();
        let acroform_id = doc.new_object_id();

        let text_field_id = doc.new_object_id();
        let checkbox_field_id = doc.new_object_id();
        let radio_parent_id = doc.new_object_id();
        let radio_widget_a_id = doc.new_object_id();
        let radio_widget_b_id = doc.new_object_id();

        let content_stream = Stream::new(dictionary! {}, Vec::new());
        doc.objects
            .insert(content_id, Object::Stream(content_stream));

        let text_field = dictionary! {
            "Type" => name("Annot"),
            "Subtype" => name("Widget"),
            "FT" => name("Tx"),
            "T" => Object::string_literal("Name"),
            "V" => Object::string_literal(""),
            "Rect" => rect(50, 700, 250, 724),
            "P" => Object::Reference(page_id),
        };
        doc.objects
            .insert(text_field_id, Object::Dictionary(text_field));

        let checkbox_field = dictionary! {
            "Type" => name("Annot"),
            "Subtype" => name("Widget"),
            "FT" => name("Btn"),
            "T" => Object::string_literal("Consent"),
            "V" => name("Off"),
            "AS" => name("Off"),
            "Rect" => rect(50, 650, 70, 670),
            "P" => Object::Reference(page_id),
            "AP" => Object::Dictionary(dictionary! {
                "N" => Object::Dictionary(dictionary! {
                    "Off" => Object::Null,
                    "Yes" => Object::Null,
                })
            }),
        };
        doc.objects
            .insert(checkbox_field_id, Object::Dictionary(checkbox_field));

        let radio_parent = dictionary! {
            "FT" => name("Btn"),
            "T" => Object::string_literal("Choice"),
            "Ff" => Object::Integer(32768),
            "Kids" => Object::Array(vec![
                Object::Reference(radio_widget_a_id),
                Object::Reference(radio_widget_b_id),
            ]),
        };
        doc.objects
            .insert(radio_parent_id, Object::Dictionary(radio_parent));

        let radio_widget_a = dictionary! {
            "Type" => name("Annot"),
            "Subtype" => name("Widget"),
            "Parent" => Object::Reference(radio_parent_id),
            "Rect" => rect(50, 600, 70, 620),
            "P" => Object::Reference(page_id),
            "AS" => name("Off"),
            "AP" => Object::Dictionary(dictionary! {
                "N" => Object::Dictionary(dictionary! {
                    "Off" => Object::Null,
                    "A" => Object::Null,
                })
            }),
        };
        doc.objects
            .insert(radio_widget_a_id, Object::Dictionary(radio_widget_a));

        let radio_widget_b = dictionary! {
            "Type" => name("Annot"),
            "Subtype" => name("Widget"),
            "Parent" => Object::Reference(radio_parent_id),
            "Rect" => rect(120, 600, 140, 620),
            "P" => Object::Reference(page_id),
            "AS" => name("Off"),
            "AP" => Object::Dictionary(dictionary! {
                "N" => Object::Dictionary(dictionary! {
                    "Off" => Object::Null,
                    "B" => Object::Null,
                })
            }),
        };
        doc.objects
            .insert(radio_widget_b_id, Object::Dictionary(radio_widget_b));

        let page = dictionary! {
            "Type" => name("Page"),
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => rect(0, 0, 595, 842),
            "Resources" => Object::Dictionary(dictionary! {}),
            "Contents" => Object::Reference(content_id),
            "Annots" => Object::Array(vec![
                Object::Reference(text_field_id),
                Object::Reference(checkbox_field_id),
                Object::Reference(radio_widget_a_id),
                Object::Reference(radio_widget_b_id),
            ]),
        };
        doc.objects.insert(page_id, Object::Dictionary(page));

        let pages = dictionary! {
            "Type" => name("Pages"),
            "Kids" => Object::Array(vec![Object::Reference(page_id)]),
            "Count" => Object::Integer(1),
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let acroform = dictionary! {
            "Fields" => Object::Array(vec![
                Object::Reference(text_field_id),
                Object::Reference(checkbox_field_id),
                Object::Reference(radio_parent_id),
            ]),
        };
        doc.objects
            .insert(acroform_id, Object::Dictionary(acroform));

        let catalog = dictionary! {
            "Type" => name("Catalog"),
            "Pages" => Object::Reference(pages_id),
            "AcroForm" => Object::Reference(acroform_id),
        };
        doc.objects
            .insert(catalog_id, Object::Dictionary(catalog));
        doc.trailer.set(b"Root", Object::Reference(catalog_id));

        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).expect("serialize fixture pdf");
        bytes
    }

    #[test]
    fn fill_blocks_errors_on_empty_pdf() {
        let fields = HashMap::<String, String>::new();
        let result = fill_blocks_impl(&[], fields);
        assert_error_code(result, "BW_PDF_EMPTY");
    }

    #[test]
    fn fill_blocks_errors_on_invalid_pdf() {
        let fields = HashMap::<String, String>::new();
        let result = fill_blocks_impl(&[1, 2, 3, 4], fields);
        assert_error_code(result, "BW_PDF_INVALID");
    }

    #[test]
    fn fill_blocks_errors_when_acroform_missing() {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let catalog_id = doc.new_object_id();
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => name("Pages"),
                "Kids" => Object::Array(vec![]),
                "Count" => Object::Integer(0),
            }),
        );
        doc.objects.insert(
            catalog_id,
            Object::Dictionary(dictionary! {
                "Type" => name("Catalog"),
                "Pages" => Object::Reference(pages_id),
            }),
        );
        doc.trailer.set(b"Root", Object::Reference(catalog_id));
        let mut input = Vec::new();
        doc.save_to(&mut input).expect("serialize minimal catalog");

        let result = fill_blocks_impl(&input, HashMap::new());
        assert_error_code(result, "BW_FORM_MISSING_ACROFORM");
    }

    #[test]
    fn fill_blocks_errors_when_no_field_names_match() {
        let pdf = make_fixture_pdf();
        let mut fields = HashMap::new();
        fields.insert("UnknownField".to_string(), "value".to_string());
        let result = fill_blocks_impl(&pdf, fields);
        assert_error_code(result, "BW_FILL_NO_MATCHING_FIELDS");
    }

    #[test]
    fn fill_blocks_errors_on_invalid_radio_value() {
        let pdf = make_fixture_pdf();
        let mut fields = HashMap::new();
        fields.insert("Choice".to_string(), "not-a-state".to_string());
        let payload = fill_blocks_impl(&pdf, fields).expect_err("invalid radio value should fail");
        assert_eq!(payload.code, "BW_FILL_BUTTON_VALUE_INVALID");
        assert_eq!(payload.context.as_deref(), Some("Choice"));
    }

    #[test]
    fn fill_blocks_updates_fixture_pdf() {
        let input_pdf = make_fixture_pdf();
        let mut fields = HashMap::new();
        fields.insert("Name".to_string(), "Ada Lovelace".to_string());
        fields.insert("Consent".to_string(), "true".to_string());
        fields.insert("Choice".to_string(), "A".to_string());

        let output_bytes = fill_blocks_impl(&input_pdf, fields).expect("fixture fields should be fillable");

        assert!(!output_bytes.is_empty(), "filled PDF payload should not be empty");
        assert_ne!(output_bytes, input_pdf, "filled PDF should differ from input bytes");
        Document::load_mem(&output_bytes).expect("filled payload should remain a valid PDF");
    }
}
