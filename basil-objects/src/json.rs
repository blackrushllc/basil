use basil_common::{Result, BasilError};

#[cfg(feature = "serde_json")]
use serde_json::{Value as JValue};

#[cfg(feature = "serde_json")]
pub fn parse_normalize(s: &str) -> Result<String> {
    let v: JValue = serde_json::from_str(s)
        .map_err(|e| BasilError(format!("JSON_PARSE$: invalid JSON: {e}")))?;
    let out = serde_json::to_string(&v)
        .map_err(|e| BasilError(format!("JSON_PARSE$: serialize failed: {e}")))?;
    Ok(out)
}

#[cfg(feature = "serde_json")]
pub fn stringify_guess(s: &str) -> Result<String> {
    if let Ok(v) = serde_json::from_str::<JValue>(s) {
        let out = serde_json::to_string(&v)
            .map_err(|e| BasilError(format!("JSON_STRINGIFY$: serialize failed: {e}")))?;
        return Ok(out);
    }
    let out = serde_json::to_string(&s)
        .map_err(|e| BasilError(format!("JSON_STRINGIFY$: wrap failed: {e}")))?;
    Ok(out)
}
