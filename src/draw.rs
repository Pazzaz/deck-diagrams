use crate::{Color, Data};
use colorous::VIRIDIS;

use svg::{
    Document, Node as _,
    node::element::{self, Rectangle},
};

pub fn create_svg(data: &Data, x_images: &[String], y_images: &[String]) -> impl svg::Node {
    let box_width: f64 = 1.4;
    let scale = 30.0;
    let x_len = data.x_values.len();
    let y_len = data.y_values.len();
    let mut min = u64::MAX;
    let mut max = u64::MIN;
    for &x in data.decks.values() {
        if x < min {
            min = x;
        }
        if x > max {
            max = x;
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
    for (i, image) in x_images.iter().enumerate() {
        let id = format!("x-{i}");

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
            .set("href", format!("data:image/jpeg;base64,{image}"))
            .set("clip-path", format!("url(#{})", id.as_str()));

        document.append(img);
        definitions.append(clip_path);
    }

    let color_width = 0.2;

    for (j, image) in y_images.iter().enumerate() {
        let info = &data.y_values[j];
        let id = format!("y-{j}");

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
            .set("href", format!("data:image/jpeg;base64,{image}"))
            .set("clip-path", format!("url(#{})", id.as_str()));

        document.append(img);
        definitions.append(clip_path);

        // Add color
        let color_str = match &info.id[..] {
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

    document.append(definitions);

    for (j, label) in data.y_values.iter().enumerate() {
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

    for (i, label) in data.x_values.iter().enumerate() {
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
    for (j, y) in data.y_values.iter().enumerate() {
        for (i, x) in data.x_values.iter().enumerate() {
            let (color, number) =
                if let Some(&x) = data.decks.get(&(x.name.clone(), y.name.clone())) {
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
        .set("style", format!("font-family:{font_family};"));

    let text_inner = text.clone().set("fill", "white");
    let text_outer1 = text.clone().set("fill", "black")
        .set("style", format!("font-family:{font_family};paint-order: stroke fill;stroke: #000000;stroke-width: 0.2;stroke-linecap: butt;stroke-linejoin: round;fill-rule: nonzero;"));
    let text_outer2 = text.set("fill", "black")
        .set("style", format!("font-family:{font_family};paint-order: stroke fill;stroke: #000000;stroke-width: 0.1;stroke-linecap: butt;stroke-linejoin: round;fill-rule: nonzero;"));
    output.append(text_outer1);
    output.append(text_outer2);
    output.append(text_inner);
}

const COLORLESS_COLOR: &str = "rgb(204.0, 194.0, 192.0)";

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            Self::White => "rgb(245, 241, 237)",
            Self::Blue => "rgb(0, 107, 167)",
            Self::Black => "rgb(60, 55, 52)",
            Self::Red => "rgb(229, 65, 43)",
            Self::Green => "rgb(0, 108, 71)",
        };
        f.write_str(x)
    }
}
