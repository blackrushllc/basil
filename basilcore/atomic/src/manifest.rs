use serde::{Deserialize, Serialize};

pub const ATOMIC_GAME_FORMAT: &str = "atomicflix-game";
pub const ATOMIC_GAME_FORMAT_VERSION: u32 = 1;
pub const ATOMIC_GAME_RUNTIME_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomicGameManifest {
    pub format: String,

    #[serde(rename = "formatVersion")]
    pub format_version: u32,

    #[serde(rename = "runtimeVersion")]
    pub runtime_version: u32,

    #[serde(rename = "gameId")]
    pub game_id: String,

    pub revision: u64,

    pub title: String,

    #[serde(rename = "programUrl")]
    pub program_url: String,

    #[serde(rename = "posterUrl")]
    pub poster_url: Option<String>,
}

impl Default for AtomicGameManifest {
    fn default() -> Self {
        Self {
            format: ATOMIC_GAME_FORMAT.to_string(),
            format_version: ATOMIC_GAME_FORMAT_VERSION,
            runtime_version: ATOMIC_GAME_RUNTIME_VERSION,
            game_id: String::new(),
            revision: 1,
            title: String::new(),
            program_url: String::new(),
            poster_url: None,
        }
    }
}
