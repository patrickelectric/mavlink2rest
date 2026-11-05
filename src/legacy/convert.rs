use serde_json::{json, Value};

use super::schema::{bitflag_entries, field_kind, is_message_name, FieldKind};

pub fn modern_to_legacy(value: &Value) -> Result<Value, String> {
    let mut out = value.clone();
    convert_value(&mut out, Direction::ModernToLegacy)?;
    Ok(out)
}

pub fn legacy_to_modern(value: &Value) -> Result<Value, String> {
    let mut out = value.clone();
    convert_value(&mut out, Direction::LegacyToModern)?;
    Ok(out)
}

#[derive(Clone, Copy)]
enum Direction {
    ModernToLegacy,
    LegacyToModern,
}

fn convert_value(value: &mut Value, direction: Direction) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(type_name)) = map.get("type").cloned() {
                if is_message_name(&type_name) && map.len() > 1 {
                    convert_message_fields(map, &type_name, direction)?;
                    return Ok(());
                }
            }
            for (_key, child) in map.iter_mut() {
                convert_value(child, direction)?;
            }
            Ok(())
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                convert_value(item, direction)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn convert_message_fields(
    map: &mut serde_json::Map<String, Value>,
    message: &str,
    direction: Direction,
) -> Result<(), String> {
    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        if key == "type" {
            continue;
        }
        let kind = match field_kind(message, &key) {
            Some(kind) => kind,
            None => {
                if let Some(child) = map.get_mut(&key) {
                    convert_value(child, direction)?;
                }
                continue;
            }
        };
        let current = map.get(&key).cloned().unwrap();
        let converted = match direction {
            Direction::ModernToLegacy => field_modern_to_legacy(message, &key, kind, current)?,
            Direction::LegacyToModern => field_legacy_to_modern(message, &key, kind, current)?,
        };
        map.insert(key, converted);
    }
    Ok(())
}

fn field_modern_to_legacy(
    message: &str,
    field: &str,
    kind: FieldKind,
    value: Value,
) -> Result<Value, String> {
    match kind {
        FieldKind::CharArray(len) => match value {
            Value::String(text) => Ok(string_to_char_array(&text, len)),
            other => Err(format!(
                "legacy convert: {message}.{field} expected string, got {other}"
            )),
        },
        FieldKind::Bitflags(enum_name) => match value {
            Value::String(text) => {
                let bits = flag_string_to_bits(enum_name, &text)?;
                Ok(json!({ "bits": bits }))
            }
            other => Err(format!(
                "legacy convert: {message}.{field} expected flag string, got {other}"
            )),
        },
    }
}

fn field_legacy_to_modern(
    message: &str,
    field: &str,
    kind: FieldKind,
    value: Value,
) -> Result<Value, String> {
    match kind {
        FieldKind::CharArray(len) => match value {
            Value::Array(items) => {
                let text = char_array_to_string(&items, len, message, field)?;
                Ok(Value::String(text))
            }
            Value::String(text) => Ok(Value::String(text)),
            other => Err(format!(
                "legacy convert: {message}.{field} expected char array, got {other}"
            )),
        },
        FieldKind::Bitflags(enum_name) => match value {
            Value::Object(map) => {
                let bits = map.get("bits").and_then(Value::as_u64).ok_or_else(|| {
                    format!("legacy convert: {message}.{field} missing bits number")
                })?;
                let text = bits_to_flag_string(enum_name, bits)?;
                Ok(Value::String(text))
            }
            Value::String(text) => Ok(Value::String(text)),
            other => Err(format!(
                "legacy convert: {message}.{field} expected {{bits: n}}, got {other}"
            )),
        },
    }
}

fn string_to_char_array(text: &str, len: usize) -> Value {
    let mut chars: Vec<Value> = text.chars().map(|c| Value::String(c.to_string())).collect();
    if chars.len() > len {
        chars.truncate(len);
    }
    while chars.len() < len {
        chars.push(Value::String(char::from(0).to_string()));
    }
    Value::Array(chars)
}

fn char_array_to_string(
    items: &[Value],
    len: usize,
    message: &str,
    field: &str,
) -> Result<String, String> {
    if items.len() != len {
        return Err(format!(
            "legacy convert: {message}.{field} expected char[{len}], got length {}",
            items.len()
        ));
    }
    let mut out = String::new();
    for (index, item) in items.iter().enumerate() {
        let ch = item
            .as_str()
            .ok_or_else(|| format!("legacy convert: {message}.{field}[{index}] is not a string"))?;
        let mut chars = ch.chars();
        let first = chars.next().unwrap_or('\0');
        if chars.next().is_some() && ch != "\u{0}" {
            // allow single char or empty; multi-char is invalid
            if ch.chars().count() != 1 {
                return Err(format!(
                    "legacy convert: {message}.{field}[{index}] expected one char, got {ch:?}"
                ));
            }
        }
        if first == '\0' {
            break;
        }
        out.push(first);
    }
    Ok(out)
}

fn flag_string_to_bits(enum_name: &str, text: &str) -> Result<u64, String> {
    let entries = bitflag_entries(enum_name)
        .ok_or_else(|| format!("legacy convert: unknown bitflag enum {enum_name}"))?;
    let text = text.trim();
    if text.is_empty() {
        return Ok(0);
    }
    let mut bits = 0u64;
    for part in text.split('|') {
        let name = part.trim();
        if name.is_empty() {
            continue;
        }
        let value = entries
            .iter()
            .find(|(entry_name, _)| *entry_name == name)
            .map(|(_, value)| *value)
            .ok_or_else(|| format!("legacy convert: unsupported flag {name} for {enum_name}"))?;
        bits |= value;
    }
    Ok(bits)
}

fn bits_to_flag_string(enum_name: &str, bits: u64) -> Result<String, String> {
    let entries = bitflag_entries(enum_name)
        .ok_or_else(|| format!("legacy convert: unknown bitflag enum {enum_name}"))?;
    if bits == 0 {
        return Ok(String::new());
    }
    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by_key(|(_, value)| std::cmp::Reverse(*value));
    let mut names = Vec::new();
    let mut used = 0u64;
    for (name, value) in sorted {
        if *value != 0 && bits & value == *value {
            names.push(*name);
            used |= value;
        }
    }
    if used != bits {
        return Err(format!(
            "legacy convert: unsupported bits {bits:#x} for {enum_name} (matched {used:#x})"
        ));
    }
    Ok(names.join(" | "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn param_set_roundtrip() {
        let modern = json!({
            "type": "PARAM_SET",
            "target_system": 1,
            "target_component": 1,
            "param_id": "SCR_FSS",
            "param_value": 1.5,
            "param_type": {"type": "MAV_PARAM_TYPE_REAL32"}
        });
        let legacy = modern_to_legacy(&modern).unwrap();
        let param_id = legacy.get("param_id").unwrap().as_array().unwrap();
        assert_eq!(param_id.len(), 16);
        assert_eq!(param_id[0], json!("S"));
        let back = legacy_to_modern(&legacy).unwrap();
        assert_eq!(back.get("param_id").unwrap(), "SCR_FSS");
    }

    #[test]
    fn heartbeat_flags_roundtrip() {
        let modern = json!({
            "type": "HEARTBEAT",
            "custom_mode": 0,
            "mavtype": {"type": "MAV_TYPE_GENERIC"},
            "autopilot": {"type": "MAV_AUTOPILOT_GENERIC"},
            "base_mode": "MAV_MODE_FLAG_SAFETY_ARMED | MAV_MODE_FLAG_CUSTOM_MODE_ENABLED",
            "system_status": {"type": "MAV_STATE_UNINIT"},
            "mavlink_version": 3
        });
        let legacy = modern_to_legacy(&modern).unwrap();
        assert_eq!(legacy["base_mode"], json!({"bits": 129}));
        let back = legacy_to_modern(&legacy).unwrap();
        let mode = back.get("base_mode").unwrap().as_str().unwrap();
        assert!(mode.contains("MAV_MODE_FLAG_SAFETY_ARMED"));
        assert!(mode.contains("MAV_MODE_FLAG_CUSTOM_MODE_ENABLED"));
    }

    #[test]
    fn unknown_flag_bits_error() {
        let legacy = json!({
            "type": "HEARTBEAT",
            "custom_mode": 0,
            "mavtype": {"type": "MAV_TYPE_GENERIC"},
            "autopilot": {"type": "MAV_AUTOPILOT_GENERIC"},
            "base_mode": {"bits": 0x100},
            "system_status": {"type": "MAV_STATE_UNINIT"},
            "mavlink_version": 3
        });
        let err = legacy_to_modern(&legacy).unwrap_err();
        assert!(err.contains("unsupported bits"));
    }
}
