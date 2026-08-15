use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

use base64::prelude::*;

use web::update_data;

use draw::create_svg;

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
            "W" => Color::White,
            "U" => Color::Blue,
            "B" => Color::Black,
            "R" => Color::Red,
            "G" => Color::Green,
            _ => return Err(()),
        };
        Ok(c)
    }
}

#[derive(Deserialize, Serialize, Clone)]
struct PartnerInfo {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset_y: Option<f64>,
    id: Vec<Color>,
}

mod companions_format {
    use super::*;
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
    x_values: Vec<PartnerInfo>,
    y_values: Vec<PartnerInfo>,

    #[serde(with = "companions_format")]
    companions: IndexMap<(String, String), u64>,
}

fn main() {
    let data_folder = PathBuf::from_str("./data/doctor_who").unwrap();
    let download_counts: bool = false;
    let save_data: bool = true;

    let f = fs::File::open(data_folder.join("data.json")).unwrap();
    let mut data: Data = serde_json::from_reader(f).unwrap();

    if download_counts {
        update_data(&mut data);
    }

    let x_positions = positions(&data.x_values);
    let y_positions = positions(&data.y_values);
    let numbers = get_counts(&x_positions, &y_positions, &data);

    if save_data {
        let mut out = fs::File::create(data_folder.join("data.json")).unwrap();

        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
        data.serialize(&mut ser).unwrap();
    }

    let x_images = get_images(&data_folder.join("images"), &data.x_values);
    let y_images = get_images(&data_folder.join("images"), &data.y_values);

    let data = draw::DrawData {
        x_names: data.x_values,
        x_images,
        y_names: data.y_values,
        y_images,
        numbers,
    };

    let svg = create_svg(&data);

    svg::save("./out2.svg", &svg).unwrap();
}

fn get_images(folder: &Path, labels: &[PartnerInfo]) -> Vec<String> {
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

fn get_counts(
    x_positions: &HashMap<String, usize>,
    y_positions: &HashMap<String, usize>,
    data: &Data,
) -> HashMap<(usize, usize), u64> {
    let mut out = HashMap::new();
    for ((x_name, y_name), count) in &data.companions {
        let x_position = x_positions.get(x_name).unwrap();
        let y_position = y_positions.get(y_name).unwrap();
        out.insert((*x_position, *y_position), *count);
    }

    out
}

fn positions(v: &[PartnerInfo]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for (i, label) in v.iter().enumerate() {
        out.insert(label.name.clone(), i);
    }
    out
}
