use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    #[serde(rename = "W")]
    White,
    #[serde(rename = "U")]
    Blue,
    #[serde(rename = "B")]
    Black,
    #[serde(rename = "R")]
    Red,
    #[serde(rename = "G")]
    Green,
}

impl FromStr for Color {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let c = match s {
            "W" => Self::White,
            "U" => Self::Blue,
            "B" => Self::Black,
            "R" => Self::Red,
            "G" => Self::Green,
            _ => return Err(()),
        };
        Ok(c)
    }
}

pub const MISSING_COLOR: colorous::Color = colorous::Color { r: 30, g: 25, b: 30 };

pub const COLORLESS_COLOR: &str = "rgb(204.0, 194.0, 192.0)";

impl Color {
    pub fn as_str(&self) -> &str {
        match self {
            Self::White => "rgb(245, 241, 237)",
            Self::Blue => "rgb(0, 107, 167)",
            Self::Black => "rgb(60, 55, 52)",
            Self::Red => "rgb(229, 65, 43)",
            Self::Green => "rgb(0, 108, 71)",
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = self.as_str();
        f.write_str(x)
    }
}
