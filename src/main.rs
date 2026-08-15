use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

use base64::prelude::*;

use web::update_data;

use draw::create_svg;

use web::WebParameters;

mod draw;
mod web;

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
enum Color {
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

#[derive(Deserialize, Serialize, Clone)]
struct Partner {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset_y: Option<f64>,
    id: Vec<Color>,
}

mod decks_format {
    use super::{Deserialize, IndexMap};
    use serde::ser::SerializeSeq;

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
struct Data {
    x_values: Vec<Partner>,
    y_values: Vec<Partner>,

    #[serde(with = "decks_format")]
    decks: IndexMap<(String, String), u64>,
}

fn main() {
    let data_folder = PathBuf::from_str("./data/doctor_who").unwrap();
    let download_counts: bool = false;
    let download_colors: bool = true;
    let save_data: bool = false;

    let f = fs::File::open(data_folder.join("data.json")).unwrap();
    let mut data: Data = serde_json::from_reader(f).unwrap();

    let to_download = WebParameters::new(download_counts, download_colors);

    if to_download.any() {
        update_data(&mut data, to_download);
    }

    if save_data {
        let mut out = fs::File::create(data_folder.join("data.json")).unwrap();

        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
        data.serialize(&mut ser).unwrap();
    }

    let x_images = get_images(&data_folder.join("images"), &data.x_values);
    let y_images = get_images(&data_folder.join("images"), &data.y_values);

    let svg = create_svg(&data, &x_images, &y_images);

    svg::save("./out2.svg", &svg).unwrap();
}

fn get_images(folder: &Path, labels: &[Partner]) -> Vec<String> {
    let mut out = Vec::new();
    let mut new = folder.to_path_buf();
    for label in labels {
        new.push(format!("{}.jpg", label.name));
        let image = load_image(&new).unwrap();
        out.push(image);
        new.pop();
    }
    out
}

// Returns base64 encoded image
fn load_image(path: &Path) -> io::Result<String> {
    let image = fs::read(path)?;
    let mut out = String::new();
    BASE64_STANDARD.encode_string(&image, &mut out);
    Ok(out)
}
