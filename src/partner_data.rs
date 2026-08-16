use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::color::Color;

#[derive(Deserialize, Serialize, Clone)]
pub struct Partner {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_y: Option<f64>,
    pub id: Vec<Color>,
}

mod decks_format {
    use serde::ser::SerializeSeq;

    use super::{Deserialize, IndexMap};

    pub fn serialize<S>(
        map: &IndexMap<(String, String), u64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(map.len()))?;
        for ((a, b), c) in map {
            seq.serialize_element(&(a, b, c))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<IndexMap<(String, String), u64>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: Vec<(String, String, u64)> = Vec::deserialize(deserializer)?;
        Ok(raw.into_iter().map(|(a, b, c)| ((a, b), c)).collect())
    }
}

#[derive(Deserialize, Serialize)]
pub struct Data {
    pub x_values: Vec<Partner>,
    pub y_values: Vec<Partner>,

    #[serde(with = "decks_format")]
    pub decks: IndexMap<(String, String), u64>,
}
