use colorous::VIRIDIS;
use regex::Regex;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io, path::{Path, PathBuf}, str::FromStr};
use svg::{
    Document, Node as _,
    node::element::{self, Rectangle},
};

use base64::prelude::*;

#[derive(Deserialize, Serialize, Clone, Copy)]
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

const COLORLESS_COLOR: &str = "rgb(204.0, 194.0, 192.0)";

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            Color::White => "rgb(245, 241, 237)",
            Color::Blue => "rgb(0, 107, 167)",
            Color::Black => "rgb(60, 55, 52)",
            Color::Red => "rgb(229, 65, 43)",
            Color::Green => "rgb(0, 108, 71)",
        };
        f.write_str(x)
    }
}

#[derive(Deserialize, Serialize, Clone)]
struct PartnerInfo {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset_y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Vec<Color>>,
}

#[derive(Deserialize, Serialize)]
struct Data {
    x_values: Vec<PartnerInfo>,
    y_values: Vec<PartnerInfo>,
    companions: Vec<(String, String, usize)>,
}

struct DrawData {
    x_names: Vec<PartnerInfo>,
    y_names: Vec<PartnerInfo>,
    x_images: Vec<String>,
    y_images: Vec<String>,
    numbers: HashMap<(usize, usize), usize>,
}

fn create_svg(data: &DrawData) -> impl svg::Node {
    let box_width: f64 = 1.4;
    let scale = 30.0;
    let x_len = data.x_names.len();
    let y_len = data.y_names.len();
    let mut min = usize::MAX;
    let mut max = usize::MIN;
    for i in 0..x_len {
        for j in 0..y_len {
            if let Some(&x) = data.numbers.get(&(i, j)) {
                if x < min {
                    min = x;
                } else if x > max {
                    max = x;
                }
            }
        }
    }

    let margin_top = 2.0;
    let margin_left = 8.0;

    let width = x_len as f64 * box_width + margin_left;
    let height = y_len as f64 + margin_top;

    let mut document = Document::new()
        .set("width", scale * width)
        .set("height", scale * height)
        .set("viewBox", (0, 0, width, height));

    let mut definitions = element::Definitions::new();

    // Add filter
    let fe_morphology = element::FilterEffectMorphology::new()
        .set("operator", "dilate")
        .set("radius", 0.1)
        .set("in", "SourceAlpha")
        .set("result", "thicken");

    let fe_flood = element::FilterEffectFlood::new().set("flood-color", "#000000");

    let fe_composite_in = element::FilterEffectComposite::new()
        .set("in2", "thicken")
        .set("operator", "in");

    let fe_composite_source = element::FilterEffectComposite::new().set("in", "SourceGraphic");

    let filter = element::Filter::new()
        .set("id", "outline")
        .add(fe_morphology)
        .add(fe_flood)
        .add(fe_composite_in)
        .add(fe_composite_source);

    definitions.append(filter);

    // Add images
    for (i, image) in data.x_images.iter().enumerate() {
        let id = format!("x-{}", i);

        let rect = Rectangle::new()
            .set("y", 0.5)
            .set("x", i as f64 * box_width + margin_left)
            .set("height", 1.51)
            .set("width", box_width)
            .set("fill", "white");
        let clip_path = element::ClipPath::new().set("id", id.as_str()).add(rect);

        let img = element::Image::new()
            .set("y", 0.5)
            .set("x", i as f64 * box_width + margin_left - (0.5 * box_width))
            .set("width", box_width * 2.0)
            .set("href", format!("data:image/jpeg;base64,{}", image))
            .set("clip-path", format!("url(#{})", id.as_str()));

        document.append(img);
        definitions.append(clip_path);
    }

    let color_width = 0.2;

    for (j, image) in data.y_images.iter().enumerate() {
        let info = &data.y_names[j];
        let id = format!("y-{}", j);

        let inner_y = margin_top + j as f64;
        let mut outer_y = margin_top + j as f64 - 3.0;

        if let Some(y_offset) = info.offset_y {
            outer_y += y_offset;
        }

        assert!(outer_y <= inner_y);

        let rect = Rectangle::new()
            .set("y", inner_y)
            .set("x", 0)
            .set("height", 1.01)
            .set("width", margin_left + 0.01)
            .set("fill", "white");
        let clip_path = element::ClipPath::new().set("id", id.as_str()).add(rect);

        let img = element::Image::new()
            .set("y", outer_y)
            .set("x", -0.5)
            .set("width", margin_left + 0.1 + 1.0)
            .set("href", format!("data:image/jpeg;base64,{}", image))
            .set("clip-path", format!("url(#{})", id.as_str()));

        document.append(img);
        definitions.append(clip_path);

        // Add color
        if let Some(color) = &info.id {
            let color_str = match &color[..] {
                [] => COLORLESS_COLOR,
                [single_color] => &single_color.to_string(),
                _ => panic!(),
            };
            let rect = Rectangle::new()
                .set("y", inner_y)
                .set("x", margin_left - color_width)
                .set("height", 1.01)
                .set("width", color_width + 0.01)
                .set("fill", color_str);
            document.append(rect);
        }
    }

    document.append(definitions);

    for (j, label) in data.y_names.iter().enumerate() {
        let name = &label.name;
        outlined_text(
            &mut document,
            "end",
            margin_left - 0.5,
            margin_top + j as f64 + 0.5,
            name,
            0.55,
            "serif",
        );
    }

    for (i, label) in data.x_names.iter().enumerate() {
        let name = label.short.as_ref().unwrap();
        outlined_text(
            &mut document,
            "middle",
            margin_left + (0.5 + i as f64) * box_width,
            margin_top - 0.7,
            name,
            0.9,
            "serif",
        );
    }

    // Draw boxes
    for j in 0..y_len {
        for i in 0..x_len {
            let (color, number) = if let Some(&x) = data.numbers.get(&(i, j)) {
                let ratio = (x - min) as f64 / (max - min) as f64;
                let scaled = 1.0 - (1.0 - ratio).powi(15);
                (VIRIDIS.eval_continuous(scaled), x.to_string())
            } else {
                (
                    colorous::Color {
                        r: 30,
                        g: 25,
                        b: 30,
                    },
                    "0".to_string(),
                )
            };
            let x_pos = i as f64 * box_width + margin_left;
            let y_pos = j as f64 + margin_top;
            let r = Rectangle::new()
                .set("x", x_pos)
                .set("y", y_pos)
                .set("width", 1.01 * box_width)
                .set("height", 1.01)
                .set(
                    "fill",
                    format!("rgb({}, {}, {})", color.r, color.g, color.b),
                );

            document.append(r);
            outlined_text(
                &mut document,
                "middle",
                x_pos + 0.5 * box_width,
                y_pos + 0.5,
                &number,
                0.5,
                "sans-serif",
            );
        }
    }

    document
}

// Returns the outline and text
fn outlined_text(
    output: &mut impl svg::Node,
    text_anchor: &str,
    x: f64,
    y: f64,
    text: &str,
    font_size: f64,
    font_family: &str,
) {
    let text = element::Text::new(text)
        .set("x", x)
        .set("y", y)
        .set("text-anchor", text_anchor)
        .set("dominant-baseline", "central")
        .set("font-size", font_size)
        .set("style", format!("font-family:{};", font_family));

    let text_inner = text.clone().set("fill", "white");
    let text_outer1 = text.clone().set("fill", "black")
        .set("style", format!("font-family:{};paint-order: stroke fill;stroke: #000000;stroke-width: 0.2;stroke-linecap: butt;stroke-linejoin: round;fill-rule: nonzero;", font_family));
    let text_outer2 = text.clone().set("fill", "black")
        .set("style", format!("font-family:{};paint-order: stroke fill;stroke: #000000;stroke-width: 0.1;stroke-linecap: butt;stroke-linejoin: round;fill-rule: nonzero;", font_family));
    output.append(text_outer1);
    output.append(text_outer2);
    output.append(text_inner);
}

fn main() {
    let data_folder = PathBuf::from_str("./data/doctor_who").unwrap();
    let download_counts: bool = false;
    let save_data: bool = true;

    let f = fs::File::open(data_folder.join("data.json")).unwrap();
    let data: Data = serde_json::from_reader(f).unwrap();

    let x_positions = positions(&data.x_values);
    let y_positions = positions(&data.y_values);

    let mut numbers = get_counts(&x_positions, &y_positions, &data);

    let client = reqwest::blocking::Client::new();

    if download_counts {
        for (i, x) in data.x_values.iter().enumerate() {
            for (j, y) in data.y_values.iter().enumerate() {
                if let Some(&c) = numbers.get(&(i, j)) {
                    if let Some(new_c) = get_deck_count(&x.name, &y.name, &client).unwrap() {
                        if new_c != c {
                            let a = slugify(&x.name);
                            let b = slugify(&y.name);
                            let url = format!("https://edhrec.com/commanders/{}-{}", a, b);
                            println!("Updated {}->{}: {}", c, new_c, url);
                            numbers.insert((i, j), new_c);
                        }
                    } else {
                        eprintln!("Parsing failed");
                    }
                }
            }
        }
    }

    if save_data {
        let mut companions = Vec::new();
        for i in 0..data.x_values.len() {
            for j in 0..data.y_values.len() {
                if let Some(&c) = numbers.get(&(i, j)) {
                    let name_x = &data.x_values[i].name;
                    let name_y = &data.y_values[j].name;
                    companions.push((name_x.clone(), name_y.clone(), c));
                }
            }
        }
        let new_data = Data {
            x_values: data.x_values.clone(),
            y_values: data.y_values.clone(),
            companions,
        };

        let mut out = fs::File::create(data_folder.join("data.json")).unwrap();

        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
        new_data.serialize(&mut ser).unwrap();
    }

    let x_images = get_images(Path::new("./data/doctor_who/images"), &data.x_values);
    let y_images = get_images(Path::new("./data/doctor_who/images"), &data.y_values);

    let data = DrawData {
        x_names: data.x_values,
        x_images,
        y_names: data.y_values,
        y_images,
        numbers,
    };

    let svg = create_svg(&data);

    svg::save("./out2.svg", &svg).unwrap();
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .replace(char::is_whitespace, "-")
        .replace(',', "")
        .replace('\'', "")
}

struct ParseData {
    deck_count: u64,
    color_0: Vec<Color>,
    color_1: Vec<Color>,
}

fn get_deck_count(
    partner1: &str,
    partner2: &str,
    client: &reqwest::blocking::Client,
) -> reqwest::Result<Option<usize>> {
    let a = slugify(partner1);
    let b = slugify(partner2);
    let url = format!("https://edhrec.com/commanders/{}-{}", a, b);
    let mut resp = client.get(&url).send()?;
    if resp.status() != StatusCode::OK {
        // Try switching the two partners in the URL
        let url = format!("https://edhrec.com/commanders/{}-{}", b, a);
        resp = client.get(&url).send()?;
        if resp.status() != StatusCode::OK {
            return Ok(None);
        }
    }

    let content = resp.text()?;

    // Extract 
    let re = Regex::new(r#"<script id="__NEXT_DATA__" type="application/json">(.+)</script>"#).unwrap();

    if let Some(capture) = re.captures(&content) {
        let block = capture.get(1).unwrap().as_str();
        let v: serde_json::Value = serde_json::from_str(block).unwrap();
        let json_dict = &v["props"]["pageProps"]["data"]["container"]["json_dict"];
        let cards = json_dict["card"]["cards"].as_array().unwrap();
        assert!(cards.len() == 2);
        let color_0 = &cards[0]["color_identity"];
        let n = json_dict["card"]["num_decks"].as_u64().unwrap();
        Ok(Some(n as usize))
    } else {
        Ok(None)
    }
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
) -> HashMap<(usize, usize), usize> {
    let mut out = HashMap::new();
    for (x_name, y_name, count) in &data.companions {
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
