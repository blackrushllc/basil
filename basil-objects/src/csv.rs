use basil_common::{Result, BasilError};

#[cfg(all(feature = "csv", feature = "serde_json"))]
use csv::{ReaderBuilder, WriterBuilder};
#[cfg(all(feature = "csv", feature = "serde_json"))]
use serde_json::{Value as JValue};

// Register into object hub; CSV provides only global helpers via VM builtins in this codebase,
// so registration is a no-op placeholder to satisfy feature wiring.
pub fn register(_reg: &mut crate::Registry) {
    // no object types to register for CSV at the moment
}

#[cfg(all(feature = "csv", feature = "serde_json"))]
#[allow(dead_code)]
pub fn parse_to_json_array(csv_text: &str) -> Result<String> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_text.as_bytes());

    let headers = rdr.headers()
        .map_err(|e| BasilError(format!("CSV_PARSE$: read headers failed: {}", e)))?
        .clone();

    let mut rows: Vec<JValue> = Vec::new();

    for rec in rdr.records() {
        let rec = rec.map_err(|e| BasilError(format!("CSV_PARSE$: read record failed: {}", e)))?;
        let mut obj = serde_json::Map::new();
        for (i, field) in rec.iter().enumerate() {
            let key = headers.get(i).unwrap_or("").to_string();
            obj.insert(key, JValue::String(field.to_string()));
        }
        rows.push(JValue::Object(obj));
    }

    let out = serde_json::to_string(&rows)
        .map_err(|e| BasilError(format!("CSV_PARSE$: serialize failed: {}", e)))?;
    Ok(out)
}

#[cfg(all(feature = "csv", feature = "serde_json"))]
#[allow(dead_code)]
pub fn write_from_rows_json(rows_json: &str) -> Result<String> {
    let rows: JValue = serde_json::from_str(rows_json)
        .map_err(|e| BasilError(format!("CSV_WRITE$: invalid JSON: {}", e)))?;

    let arr = rows.as_array().ok_or_else(|| BasilError("CSV_WRITE$: expected JSON array of objects".into()))?;

    let mut headers: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Some(first) = arr.first().and_then(|v| v.as_object()) {
        for k in first.keys() {
            headers.push(k.clone());
            seen.insert(k.clone());
        }
    }
    for v in arr.iter() {
        if let Some(obj) = v.as_object() {
            for k in obj.keys() {
                if !seen.contains(k) {
                    headers.push(k.clone());
                    seen.insert(k.clone());
                }
            }
        }
    }

    let mut wtr = WriterBuilder::new().from_writer(vec![]);
    // write headers
    wtr.write_record(headers.iter())
        .map_err(|e| BasilError(format!("CSV_WRITE$: write headers failed: {}", e)))?;

    // write rows
    for v in arr.iter() {
        let obj = v.as_object().ok_or_else(|| BasilError("CSV_WRITE$: array items must be objects".into()))?;
        let mut row: Vec<String> = Vec::with_capacity(headers.len());
        for h in headers.iter() {
            let cell = match obj.get(h) {
                Some(JValue::String(s)) => s.clone(),
                Some(JValue::Number(n)) => n.to_string(),
                Some(JValue::Bool(b)) => if *b { "true".to_string() } else { "false".to_string() },
                Some(JValue::Null) => String::new(),
                Some(other) => serde_json::to_string(other).unwrap_or_default(),
                None => String::new(),
            };
            row.push(cell);
        }
        wtr.write_record(&row).map_err(|e| BasilError(format!("CSV_WRITE$: write row failed: {}", e)))?;
    }

    let bytes = wtr.into_inner().map_err(|e| BasilError(format!("CSV_WRITE$: finalize failed: {}", e)))?;
    let out = String::from_utf8(bytes).map_err(|e| BasilError(format!("CSV_WRITE$: utf8 failed: {}", e)))?;
    Ok(out)
}
