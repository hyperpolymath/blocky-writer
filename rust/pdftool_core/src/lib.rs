// SPDX-License-Identifier: MPL-2.0 WITH Palimpsest

#![forbid(unsafe_code)]
use std::collections::{HashMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
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
        Object::Reference(id) => doc.get_object(*id).cloned(),
        _ => Ok(obj.clone()),
    }
}

fn resolve_object_chain(doc: &Document, obj: &Object) -> Option<Object> {
    let mut resolved = obj.clone();
    let mut references = HashSet::<ObjectId>::new();

    loop {
        match resolved {
            Object::Reference(id) => {
                if !references.insert(id) {
                    return None;
                }
                resolved = doc.get_object(id).ok()?.clone();
            }
            value => return Some(value),
        }
    }
}

/// Resolves a PDF number, returning `None` for unresolved or cyclic references,
/// non-numbers, non-finite reals, or integers that cannot be represented exactly.
fn object_to_number(doc: &Document, obj: &Object) -> Option<f64> {
    match resolve_object_chain(doc, obj)? {
        Object::Integer(value) => {
            let converted = value as f64;
            // Reject integers outside f64's exact range rather than
            // silently presenting rounded coordinates to callers.
            ((converted as i128) == (value as i128)).then_some(converted)
        }
        Object::Real(value) => {
            let converted = f64::from(value);
            converted.is_finite().then_some(converted)
        }
        _ => None,
    }
}

/// Converts two diagonally opposite corners in either order to a rectangle
/// `(x, y, width, height)`, using the smaller coordinates as the origin. Zero
/// extents are accepted; non-finite coordinates or extents return `None`.
fn rect_from_coordinates(x1: f64, y1: f64, x2: f64, y2: f64) -> Option<(f64, f64, f64, f64)> {
    if ![x1, y1, x2, y2]
        .iter()
        .all(|coordinate| coordinate.is_finite())
    {
        return None;
    }

    // PDF Rect stores two diagonally opposite corners, which may be supplied
    // in either order. Canonicalize the pair ordering required to expose an
    // x/y/width/height block; this is PDF-defined interpretation, not a page
    // coordinate transform.
    let llx = x1.min(x2);
    let lly = y1.min(y2);
    let urx = x1.max(x2);
    let ury = y1.max(y2);
    let width = urx - llx;
    let height = ury - lly;

    if !width.is_finite() || !height.is_finite() {
        return None;
    }

    Some((llx, lly, width, height))
}

/// Converts a direct four-element PDF rectangle array to `(x, y, width, height)`.
/// Coordinates may be indirect. Other objects, invalid numbers or non-finite
/// extents yield `None`.
fn rect_from_object(doc: &Document, obj: &Object) -> Option<(f64, f64, f64, f64)> {
    let Object::Array(values) = obj else {
        return None;
    };

    if values.len() != 4 {
        return None;
    }

    rect_from_coordinates(
        object_to_number(doc, &values[0])?,
        object_to_number(doc, &values[1])?,
        object_to_number(doc, &values[2])?,
        object_to_number(doc, &values[3])?,
    )
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

fn get_dict<'a>(
    doc: &'a Document,
    id: ObjectId,
    code: &'static str,
    context: &str,
) -> CoreResult<&'a Dictionary> {
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
    let root = doc.trailer.get(b"Root").map_err(|err| {
        core_error_with_context(
            "BW_PDF_ROOT_MISSING",
            err.to_string(),
            Some("trailer.Root".into()),
        )
    })?;
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
        let catalog = get_dict(
            doc,
            catalog_id,
            "BW_FORM_CATALOG_INVALID",
            "catalog dictionary",
        )?;
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
            let catalog = get_dict_mut(
                doc,
                catalog_id,
                "BW_FORM_CATALOG_INVALID",
                "catalog dictionary",
            )?;
            catalog.set(b"AcroForm", Object::Reference(form_id));
            Ok(form_id)
        }
    }
}

fn collect_field_ids(
    doc: &Document,
    source: &Object,
    out: &mut Vec<ObjectId>,
    seen: &mut HashSet<ObjectId>,
) {
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

fn collect_widget_ids_for_field(
    doc: &Document,
    source: &Object,
    out: &mut Vec<ObjectId>,
    seen: &mut HashSet<ObjectId>,
) {
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

fn field_input_value(
    descriptor: &FieldDescriptor,
    fields: &HashMap<String, String>,
) -> Option<String> {
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

fn set_field_text_value(
    doc: &mut Document,
    descriptor: &FieldDescriptor,
    value: &str,
) -> CoreResult<()> {
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

fn set_button_value(
    doc: &mut Document,
    descriptor: &FieldDescriptor,
    raw_value: &str,
) -> CoreResult<()> {
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
                format!(
                    "button value '{}' does not match available widget states",
                    raw_value
                ),
                descriptor
                    .full_name
                    .clone()
                    .or(descriptor.partial_name.clone()),
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
                let widget_value =
                    widget_on_state(doc, *widget_id).unwrap_or_else(|| field_value.clone());
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

fn apply_field_value(
    doc: &mut Document,
    descriptor: &FieldDescriptor,
    value: &str,
) -> CoreResult<()> {
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
            descriptor
                .full_name
                .clone()
                .or(descriptor.partial_name.clone()),
        )),
    }
}

/// Detects widget annotations in PDF page order and returns their labels and
/// rectangles in PDF user space. Corner order is normalised, and
/// missing labels fall back to `field_<page>_<annotation>`.
///
/// Missing, null or unresolved annotation entries and non-widget annotations
/// are skipped. Returns an error for empty or unreadable PDFs, unreadable or
/// invalid pages, non-array `Annots`, non-dictionary annotations, or missing,
/// unresolved or invalid widget rectangles.
fn detect_blocks_impl(pdf_data: &[u8]) -> CoreResult<Vec<Block>> {
    if pdf_data.is_empty() {
        return Err(core_error("BW_PDF_EMPTY", "empty PDF payload"));
    }

    let doc = Document::load_mem(pdf_data).map_err(|err| {
        core_error_with_context(
            "BW_PDF_INVALID",
            err.to_string(),
            Some("Document::load_mem".into()),
        )
    })?;

    let mut blocks = Vec::<Block>::new();
    for (page_number, page_id) in doc.get_pages() {
        let page_obj = doc.get_object(page_id).map_err(|err| {
            core_error_with_context(
                "BW_PDF_PAGE_READ_FAILED",
                err.to_string(),
                Some(format!("page {}", page_number)),
            )
        })?;
        let page_dict = page_obj.as_dict().map_err(|err| {
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
            // PDF null entries, including references to missing objects, have
            // the same effect as an absent optional /Annots entry.
            Ok(Object::Null) | Err(_) => continue,
            Ok(_) => {
                return Err(core_error_with_context(
                    "BW_PDF_ANNOTATIONS_INVALID",
                    "page Annots entry is not an array",
                    Some(format!("page {}", page_number)),
                ));
            }
        };

        for (annot_index, annot_ref) in annots.iter().enumerate() {
            let annot_obj = match resolve_object(&doc, annot_ref) {
                Ok(Object::Null) | Err(_) => continue,
                Ok(obj) => obj,
            };
            let widget = match annot_obj {
                Object::Dictionary(dict) => dict,
                Object::Null => continue,
                _ => {
                    return Err(core_error_with_context(
                        "BW_PDF_ANNOTATION_INVALID",
                        "page annotation is not a dictionary",
                        Some(format!(
                            "page {} annotation {}",
                            page_number,
                            annot_index + 1
                        )),
                    ));
                }
            };

            let is_widget = is_widget_dict(&widget);
            if !is_widget {
                continue;
            }

            let rect_obj = widget.get(b"Rect").map_err(|err| {
                core_error_with_context(
                    "BW_PDF_WIDGET_RECT_INVALID",
                    format!("widget Rect is missing: {err}"),
                    Some(format!(
                        "page {} annotation {}",
                        page_number,
                        annot_index + 1
                    )),
                )
            })?;
            let rect_obj = resolve_object_chain(&doc, rect_obj).ok_or_else(|| {
                core_error_with_context(
                    "BW_PDF_WIDGET_RECT_INVALID",
                    "could not resolve widget Rect or encountered an indirect-reference cycle",
                    Some(format!(
                        "page {} annotation {}",
                        page_number,
                        annot_index + 1
                    )),
                )
            })?;
            let rect = rect_from_object(&doc, &rect_obj).ok_or_else(|| {
                core_error_with_context(
                    "BW_PDF_WIDGET_RECT_INVALID",
                    "widget Rect must contain four finite, exactly representable numbers with finite extents",
                    Some(format!(
                        "page {} annotation {}",
                        page_number,
                        annot_index + 1
                    )),
                )
            })?;

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

    let mut doc = Document::load_mem(pdf_data).map_err(|err| {
        core_error_with_context(
            "BW_PDF_INVALID",
            err.to_string(),
            Some("Document::load_mem".into()),
        )
    })?;

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
        acroform
            .get(b"Fields")
            .map_err(|err| {
                core_error_with_context(
                    "BW_FORM_FIELDS_MISSING",
                    err.to_string(),
                    Some("AcroForm.Fields".into()),
                )
            })?
            .clone()
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
    doc.save_to(&mut output).map_err(|err| {
        core_error_with_context(
            "BW_FILL_SAVE_FAILED",
            err.to_string(),
            Some("Document::save_to".into()),
        )
    })?;

    Ok(output)
}

#[wasm_bindgen]
pub fn detect_blocks(pdf_data: &[u8]) -> Result<JsValue, JsValue> {
    let blocks = detect_blocks_impl(pdf_data).map_err(core_error_to_js)?;
    serde_wasm_bindgen::to_value(&blocks).map_err(|err| {
        core_error_to_js(core_error_with_context(
            "BW_SERIALIZATION_ERROR",
            err.to_string(),
            Some("detect_blocks".into()),
        ))
    })
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

    let field_values: HashMap<String, String> =
        serde_wasm_bindgen::from_value(fields).map_err(|err| {
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

    fn assert_error_code<T: std::fmt::Debug>(result: CoreResult<T>, expected_code: &str) {
        let payload = result.expect_err("expected operation to fail");
        assert_eq!(payload.code, expected_code);
        assert!(
            !payload.message.is_empty(),
            "error message should not be empty"
        );
    }

    fn save_fixture_doc(doc: &mut Document) -> Vec<u8> {
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).expect("serialize fixture PDF");
        bytes
    }

    fn fixture_with_first_widget_rect(pdf_data: &[u8], rect: Object) -> Vec<u8> {
        let mut doc = Document::load_mem(pdf_data).expect("load fixture PDF");
        let page_id = *doc
            .get_pages()
            .values()
            .next()
            .expect("fixture has one page");
        let widget_id = {
            let page = doc
                .get_object(page_id)
                .expect("fixture page exists")
                .as_dict()
                .expect("fixture page is a dictionary");
            let annots = page
                .get(b"Annots")
                .expect("fixture page has Annots")
                .as_array()
                .expect("fixture Annots is an array");
            match annots.first().expect("fixture has a widget annotation") {
                Object::Reference(id) => *id,
                _ => panic!("fixture widget must be indirect"),
            }
        };
        doc.get_object_mut(widget_id)
            .expect("fixture widget exists")
            .as_dict_mut()
            .expect("fixture widget is a dictionary")
            .set(b"Rect", rect);
        save_fixture_doc(&mut doc)
    }

    fn fixture_with_first_widget_indirect_rect(pdf_data: &[u8], rect: Object) -> Vec<u8> {
        let mut doc = Document::load_mem(pdf_data).expect("load fixture PDF");
        let page_id = *doc
            .get_pages()
            .values()
            .next()
            .expect("fixture has one page");
        let widget_id = {
            let page = doc
                .get_object(page_id)
                .expect("fixture page exists")
                .as_dict()
                .expect("fixture page is a dictionary");
            let annots = page
                .get(b"Annots")
                .expect("fixture page has Annots")
                .as_array()
                .expect("fixture Annots is an array");
            match annots.first().expect("fixture has a widget annotation") {
                Object::Reference(id) => *id,
                _ => panic!("fixture widget must be indirect"),
            }
        };
        let rect_id = doc.new_object_id();
        doc.objects.insert(rect_id, rect);
        doc.get_object_mut(widget_id)
            .expect("fixture widget exists")
            .as_dict_mut()
            .expect("fixture widget is a dictionary")
            .set(b"Rect", Object::Reference(rect_id));
        save_fixture_doc(&mut doc)
    }

    fn fixture_with_first_widget_chained_indirect_rect(pdf_data: &[u8], rect: Object) -> Vec<u8> {
        let indirect_pdf = fixture_with_first_widget_indirect_rect(pdf_data, rect);
        let mut doc = Document::load_mem(&indirect_pdf).expect("load indirect-Rect fixture");
        let page_id = *doc
            .get_pages()
            .values()
            .next()
            .expect("fixture has one page");
        let widget_id = {
            let page = doc
                .get_object(page_id)
                .expect("fixture page exists")
                .as_dict()
                .expect("fixture page is a dictionary");
            let annots = page
                .get(b"Annots")
                .expect("fixture page has Annots")
                .as_array()
                .expect("fixture Annots is an array");
            match annots.first().expect("fixture has a widget annotation") {
                Object::Reference(id) => *id,
                _ => panic!("fixture widget must be indirect"),
            }
        };
        let rect_id = {
            let widget = doc
                .get_object(widget_id)
                .expect("fixture widget exists")
                .as_dict()
                .expect("fixture widget is a dictionary");
            match widget.get(b"Rect").expect("fixture widget has Rect") {
                Object::Reference(id) => *id,
                _ => panic!("fixture Rect must be indirect"),
            }
        };
        let chained_id = doc.new_object_id();
        doc.objects.insert(chained_id, Object::Reference(rect_id));
        doc.get_object_mut(widget_id)
            .expect("fixture widget exists")
            .as_dict_mut()
            .expect("fixture widget is a dictionary")
            .set(b"Rect", Object::Reference(chained_id));
        save_fixture_doc(&mut doc)
    }

    fn fixture_with_first_widget_cyclic_indirect_rect(pdf_data: &[u8]) -> Vec<u8> {
        let mut doc = Document::load_mem(pdf_data).expect("load fixture PDF");
        let page_id = *doc
            .get_pages()
            .values()
            .next()
            .expect("fixture has one page");
        let widget_id = {
            let page = doc
                .get_object(page_id)
                .expect("fixture page exists")
                .as_dict()
                .expect("fixture page is a dictionary");
            let annots = page
                .get(b"Annots")
                .expect("fixture page has Annots")
                .as_array()
                .expect("fixture Annots is an array");
            match annots.first().expect("fixture has a widget annotation") {
                Object::Reference(id) => *id,
                _ => panic!("fixture widget must be indirect"),
            }
        };
        let first_id = doc.new_object_id();
        let second_id = doc.new_object_id();
        doc.objects.insert(first_id, Object::Reference(second_id));
        doc.objects.insert(second_id, Object::Reference(first_id));
        doc.get_object_mut(widget_id)
            .expect("fixture widget exists")
            .as_dict_mut()
            .expect("fixture widget is a dictionary")
            .set(b"Rect", Object::Reference(first_id));
        save_fixture_doc(&mut doc)
    }

    fn fixture_with_first_widget_indirect_rect_coordinates(pdf_data: &[u8]) -> Vec<u8> {
        let mut doc = Document::load_mem(pdf_data).expect("load fixture PDF");
        let page_id = *doc
            .get_pages()
            .values()
            .next()
            .expect("fixture has one page");
        let widget_id = {
            let page = doc
                .get_object(page_id)
                .expect("fixture page exists")
                .as_dict()
                .expect("fixture page is a dictionary");
            let annots = page
                .get(b"Annots")
                .expect("fixture page has Annots")
                .as_array()
                .expect("fixture Annots is an array");
            match annots.first().expect("fixture has a widget annotation") {
                Object::Reference(id) => *id,
                _ => panic!("fixture widget must be indirect"),
            }
        };
        let mut coordinates = Vec::new();
        for value in [50_i64, 700, 250, 724] {
            let id = doc.new_object_id();
            doc.objects.insert(id, Object::Integer(value));
            coordinates.push(Object::Reference(id));
        }
        doc.get_object_mut(widget_id)
            .expect("fixture widget exists")
            .as_dict_mut()
            .expect("fixture widget is a dictionary")
            .set(b"Rect", Object::Array(coordinates));
        save_fixture_doc(&mut doc)
    }

    fn fixture_without_first_widget_rect(pdf_data: &[u8]) -> Vec<u8> {
        let mut doc = Document::load_mem(pdf_data).expect("load fixture PDF");
        let page_id = *doc
            .get_pages()
            .values()
            .next()
            .expect("fixture has one page");
        let widget_id = {
            let page = doc
                .get_object(page_id)
                .expect("fixture page exists")
                .as_dict()
                .expect("fixture page is a dictionary");
            let annots = page
                .get(b"Annots")
                .expect("fixture page has Annots")
                .as_array()
                .expect("fixture Annots is an array");
            match annots.first().expect("fixture has a widget annotation") {
                Object::Reference(id) => *id,
                _ => panic!("fixture widget must be indirect"),
            }
        };
        let widget = dictionary! {
            "Type" => name("Annot"),
            "Subtype" => name("Widget"),
        };
        doc.objects.insert(widget_id, Object::Dictionary(widget));
        save_fixture_doc(&mut doc)
    }

    fn fixture_with_page_annots(pdf_data: &[u8], annots: Object) -> Vec<u8> {
        let mut doc = Document::load_mem(pdf_data).expect("load fixture PDF");
        let page_id = *doc
            .get_pages()
            .values()
            .next()
            .expect("fixture has one page");
        doc.get_object_mut(page_id)
            .expect("fixture page exists")
            .as_dict_mut()
            .expect("fixture page is a dictionary")
            .set(b"Annots", annots);
        save_fixture_doc(&mut doc)
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
        doc.objects.insert(catalog_id, Object::Dictionary(catalog));
        doc.trailer.set(b"Root", Object::Reference(catalog_id));

        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).expect("serialize fixture pdf");
        bytes
    }

    #[test]
    fn detect_blocks_returns_widget_rectangles_in_pdf_user_space() {
        let pdf = make_fixture_pdf();
        let blocks = detect_blocks_impl(&pdf).expect("valid widget rectangles should detect");

        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0].label, "Name");
        assert_eq!((blocks[0].x, blocks[0].y), (50.0, 700.0));
        assert_eq!((blocks[0].width, blocks[0].height), (200.0, 24.0));
        assert_eq!(blocks[1].label, "Consent");
        assert_eq!(blocks[2].label, "Choice");
        assert_eq!(blocks[3].label, "Choice");
        assert_eq!((blocks[3].x, blocks[3].y), (120.0, 600.0));
    }

    #[test]
    fn detect_blocks_resolves_indirect_widget_rectangles() {
        let base = make_fixture_pdf();
        let pdf = fixture_with_first_widget_indirect_rect(&base, rect(50, 700, 250, 724));
        let blocks = detect_blocks_impl(&pdf).expect("indirect Rect arrays should resolve");
        assert_eq!((blocks[0].x, blocks[0].y), (50.0, 700.0));
        assert_eq!((blocks[0].width, blocks[0].height), (200.0, 24.0));

        let chained_pdf =
            fixture_with_first_widget_chained_indirect_rect(&base, rect(50, 700, 250, 724));
        let chained_blocks =
            detect_blocks_impl(&chained_pdf).expect("chained indirect Rect arrays should resolve");
        assert_eq!((chained_blocks[0].x, chained_blocks[0].y), (50.0, 700.0));
        assert_eq!(
            (chained_blocks[0].width, chained_blocks[0].height),
            (200.0, 24.0)
        );
    }

    #[test]
    fn detect_blocks_resolves_indirect_widget_rect_coordinates() {
        let pdf = fixture_with_first_widget_indirect_rect_coordinates(&make_fixture_pdf());
        let blocks = detect_blocks_impl(&pdf).expect("indirect Rect coordinates should resolve");
        assert_eq!((blocks[0].x, blocks[0].y), (50.0, 700.0));
        assert_eq!((blocks[0].width, blocks[0].height), (200.0, 24.0));
    }

    #[test]
    fn detect_blocks_returns_empty_for_page_without_annotations() {
        let pdf = fixture_with_page_annots(&make_fixture_pdf(), Object::Array(Vec::new()));
        let blocks = detect_blocks_impl(&pdf).expect("empty Annots array is valid");
        assert!(blocks.is_empty());
    }

    #[test]
    fn detect_blocks_rejects_non_array_page_annots() {
        let pdf = fixture_with_page_annots(&make_fixture_pdf(), name("not-an-array"));
        assert_error_code(detect_blocks_impl(&pdf), "BW_PDF_ANNOTATIONS_INVALID");
    }

    #[test]
    fn detect_blocks_treats_null_and_unresolved_page_annots_as_absent() {
        let base = make_fixture_pdf();
        let null_annots = fixture_with_page_annots(&base, Object::Null);
        assert!(detect_blocks_impl(&null_annots)
            .expect("null Annots has the effect of an absent entry")
            .is_empty());

        let missing_id = (u32::MAX, u16::MAX);
        let unresolved_annots = fixture_with_page_annots(&base, Object::Reference(missing_id));
        assert!(detect_blocks_impl(&unresolved_annots)
            .expect("unresolved Annots is treated as null")
            .is_empty());
    }

    #[test]
    fn detect_blocks_normalizes_pdf_rect_corner_order() {
        let pdf = fixture_with_first_widget_rect(&make_fixture_pdf(), rect(250, 724, 50, 700));
        let blocks = detect_blocks_impl(&pdf).expect("either diagonal-corner order is valid");
        assert_eq!((blocks[0].x, blocks[0].y), (50.0, 700.0));
        assert_eq!((blocks[0].width, blocks[0].height), (200.0, 24.0));
    }

    #[test]
    fn detect_blocks_rejects_malformed_widget_rectangles() {
        let malformed_length = Object::Array(vec![
            Object::Integer(50),
            Object::Integer(700),
            Object::Integer(250),
        ]);
        let pdf = fixture_with_first_widget_rect(&make_fixture_pdf(), malformed_length);
        assert_error_code(detect_blocks_impl(&pdf), "BW_PDF_WIDGET_RECT_INVALID");

        let malformed_type = Object::Array(vec![
            Object::Integer(50),
            name("not-a-number"),
            Object::Integer(250),
            Object::Integer(724),
        ]);
        let pdf = fixture_with_first_widget_rect(&make_fixture_pdf(), malformed_type);
        assert_error_code(detect_blocks_impl(&pdf), "BW_PDF_WIDGET_RECT_INVALID");
    }

    #[test]
    fn object_to_number_preserves_f64_integer_boundaries() {
        let mut doc = Document::with_version("1.7");

        assert_eq!(
            object_to_number(&doc, &Object::Integer(16_777_217)),
            Some(16_777_217.0)
        );
        assert_eq!(
            object_to_number(&doc, &Object::Integer(9_007_199_254_740_992)),
            Some(9_007_199_254_740_992.0)
        );
        assert_eq!(
            object_to_number(&doc, &Object::Integer(i64::MIN)),
            Some(i64::MIN as f64)
        );
        assert_eq!(
            object_to_number(&doc, &Object::Integer(9_007_199_254_740_993)),
            None
        );

        let inner_id = doc.new_object_id();
        doc.objects.insert(inner_id, Object::Integer(42));
        let outer_id = doc.new_object_id();
        doc.objects.insert(outer_id, Object::Reference(inner_id));
        assert_eq!(
            object_to_number(&doc, &Object::Reference(outer_id)),
            Some(42.0)
        );

        let first_id = doc.new_object_id();
        let second_id = doc.new_object_id();
        doc.objects.insert(first_id, Object::Reference(second_id));
        doc.objects.insert(second_id, Object::Reference(first_id));
        assert_eq!(object_to_number(&doc, &Object::Reference(first_id)), None);
        assert_eq!(object_to_number(&doc, &Object::Real(f32::NAN)), None);
        assert_eq!(object_to_number(&doc, &Object::Real(f32::INFINITY)), None);
    }

    #[test]
    fn rect_from_object_rejects_non_finite_values_and_overflowing_extents() {
        let doc = Document::with_version("1.7");
        let non_finite = Object::Array(vec![
            Object::Real(f32::NAN),
            Object::Integer(0),
            Object::Integer(10),
            Object::Integer(10),
        ]);
        assert_eq!(rect_from_object(&doc, &non_finite), None);

        assert_eq!(rect_from_coordinates(-f64::MAX, 0.0, f64::MAX, 1.0), None);
        assert_eq!(rect_from_coordinates(0.0, -f64::MAX, 1.0, f64::MAX), None);
        assert_eq!(
            rect_from_object(&doc, &rect(16_777_217, 0, 16_777_250, 1)),
            Some((16_777_217.0, 0.0, 33.0, 1.0))
        );
    }

    #[test]
    fn detect_blocks_reports_missing_widget_rectangles() {
        let pdf = fixture_without_first_widget_rect(&make_fixture_pdf());
        assert_error_code(detect_blocks_impl(&pdf), "BW_PDF_WIDGET_RECT_INVALID");
    }

    #[test]
    fn detect_blocks_reports_unresolved_widget_rect_references() {
        let missing_id = (u32::MAX, u16::MAX);
        let unresolved_array =
            fixture_with_first_widget_rect(&make_fixture_pdf(), Object::Reference(missing_id));
        assert_error_code(
            detect_blocks_impl(&unresolved_array),
            "BW_PDF_WIDGET_RECT_INVALID",
        );

        let unresolved_coordinate = fixture_with_first_widget_rect(
            &make_fixture_pdf(),
            Object::Array(vec![
                Object::Reference(missing_id),
                Object::Integer(0),
                Object::Integer(10),
                Object::Integer(10),
            ]),
        );
        assert_error_code(
            detect_blocks_impl(&unresolved_coordinate),
            "BW_PDF_WIDGET_RECT_INVALID",
        );

        let cyclic_rect = fixture_with_first_widget_cyclic_indirect_rect(&make_fixture_pdf());
        assert_error_code(
            detect_blocks_impl(&cyclic_rect),
            "BW_PDF_WIDGET_RECT_INVALID",
        );
    }

    #[test]
    fn detect_blocks_accepts_integer_coordinates_above_f32_precision_limit() {
        let precise_rect = rect(16_777_217, 700, 16_777_250, 724);
        let pdf = fixture_with_first_widget_rect(&make_fixture_pdf(), precise_rect);
        let blocks = detect_blocks_impl(&pdf).expect("f64 preserves these valid PDF integers");
        assert_eq!(blocks[0].x, 16_777_217.0);
        assert_eq!(blocks[0].width, 33.0);
    }

    #[test]
    fn detect_blocks_skips_null_and_unresolved_annotation_entries() {
        let missing_id = (u32::MAX, u16::MAX);
        let pdf = fixture_with_page_annots(
            &make_fixture_pdf(),
            Object::Array(vec![Object::Null, Object::Reference(missing_id)]),
        );
        assert!(detect_blocks_impl(&pdf)
            .expect("null and unresolved annotation entries are absent")
            .is_empty());
    }

    #[test]
    fn detect_blocks_ignores_valid_non_widget_annotations() {
        let annotation = Object::Dictionary(dictionary! {
            "Type" => name("Annot"),
            "Subtype" => name("Text"),
            "Rect" => rect(50, 700, 250, 724),
        });
        let pdf = fixture_with_page_annots(&make_fixture_pdf(), Object::Array(vec![annotation]));
        let blocks = detect_blocks_impl(&pdf).expect("non-widget annotations are valid");
        assert!(blocks.is_empty());
    }

    #[test]
    fn detect_blocks_reports_non_dictionary_annotations() {
        let pdf = fixture_with_page_annots(
            &make_fixture_pdf(),
            Object::Array(vec![name("not-a-dictionary")]),
        );
        assert_error_code(detect_blocks_impl(&pdf), "BW_PDF_ANNOTATION_INVALID");
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

        let output_bytes =
            fill_blocks_impl(&input_pdf, fields).expect("fixture fields should be fillable");

        assert!(
            !output_bytes.is_empty(),
            "filled PDF payload should not be empty"
        );
        assert_ne!(
            output_bytes, input_pdf,
            "filled PDF should differ from input bytes"
        );
        Document::load_mem(&output_bytes).expect("filled payload should remain a valid PDF");
    }
}
