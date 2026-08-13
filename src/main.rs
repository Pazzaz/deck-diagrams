use colorous::VIRIDIS;
use serde::Deserialize;
use std::{collections::HashMap, fs, io, path::Path};
use svg::{
    Document, Node as _, node::element::{self, Rectangle},
};

use base64::prelude::*;

#[derive(Deserialize)]
enum Color {
    #[serde(rename = "w")]
    White,
    #[serde(rename = "u")]
    Blue,
    #[serde(rename = "b")]
    Black,
    #[serde(rename = "r")]
    Red,
    #[serde(rename = "g")]
    Green,
    #[serde(rename = "c")]
    Colorless,
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            Color::White => "rgb(255.0, 251.0, 213.0)",
            Color::Blue => "rgb(170.0, 224.0, 250.0)",
            Color::Black => "rgb(19.0, 12.0, 14.0)",
            Color::Red => "rgb(249.0, 170.0, 143.0)",
            Color::Green => "rgb(155.0, 211.0, 174.0)",
            Color::Colorless => "rgb(204.0, 194.0, 192.0)",
        };
        f.write_str(x)
    }
}

#[derive(Deserialize)]
struct Value {
    name: String,
    short: Option<String>,
    companions: Option<Vec<(String, usize)>>,
    offset_y: Option<f64>,
    id: Option<Color>,
}

#[derive(Deserialize)]
struct Data {
    x_values: Vec<Value>,
    y_values: Vec<Value>,
}

struct DrawData {
    x_names: Vec<Value>,
    y_names: Vec<Value>,
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

    let fe_flood = element::FilterEffectFlood::new()
        .set("flood-color", "#000000");

    let fe_composite_in = element::FilterEffectComposite::new()
        .set("in2", "thicken")
        .set("operator", "in");

    let fe_composite_source = element::FilterEffectComposite::new()
        .set("in", "SourceGraphic");

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
            let rect = Rectangle::new()
                .set("y", inner_y)
                .set("x", margin_left - color_width)
                .set("height", 1.01)
                .set("width", color_width + 0.01)
                .set("fill", color.to_string());
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
    let f = fs::File::open("./data/doctor_who/data.json").unwrap();
    let data: Data = serde_json::from_reader(f).unwrap();

    let x_positions = positions(&data.x_values);
    let y_positions = positions(&data.y_values);

    let numbers = get_counts(&x_positions, &y_positions, &data);

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

    // let top: String = iter::once("".to_string()).chain(ALL_DOCTORS.iter().map(|x| x.to_string())).join("\t");
    // println!("{}", top);
    // for i in 0..companion_names.len() {
    //     print!(r#"{}"#, companion_names[i]);
    //     for doctor in ALL_DOCTORS {
    //         let count = counts.get(&(doctor, i)).unwrap_or(&0);
    //         print!("\t{}", count);
    //     }
    //     println!();
    // }
}

fn get_images(folder: &Path, labels: &[Value]) -> Vec<String> {
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
    for label in &data.x_values {
        let x_position = x_positions.get(&label.name).unwrap();
        for (partner_name, count) in label.companions.as_ref().unwrap() {
            let partner_i = y_positions.get(partner_name).unwrap();
            out.insert((*x_position, *partner_i), *count);
        }
    }

    out
}

fn positions(v: &[Value]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for (i, label) in v.iter().enumerate() {
        out.insert(label.name.clone(), i);
    }
    out
}

// use regex::Regex;
// fn parse_html(html: &str) -> Vec<(&str, usize)> {
//     let re = Regex::new(r#">([^<(]+) \(([^)]+)\)<"#).unwrap();
//     let mut out = Vec::new();

//     for (_, [name, count]) in re.captures_iter(html).map(|c| c.extract()) {
//         out.push((name, count.parse::<usize>().unwrap()));
//     }
//     out
// }
