use base64::prelude::*;
use colorous::VIRIDIS;
use indexmap::IndexMap;
use svg::{
    Document, Node as _,
    node::element::{self, Rectangle},
};

use crate::{
    color::{COLORLESS_COLOR, Color, MISSING_COLOR},
    partner_data::Partner,
};

#[derive(Debug, Clone, Copy)]
struct ImageParams<'a> {
    x: f64,
    y: f64,
    height: f64,
    width: f64,
    x_padding: f64,
    y_padding: f64,
    id: &'a str,
    image: &'a str,
}

fn add_image(
    document: &mut impl svg::Node,
    definitions: &mut element::Definitions,
    params: ImageParams,
) {
    let rect = Rectangle::new()
        .set("x", params.x)
        .set("y", params.y)
        .set("width", params.width)
        .set("height", params.height)
        .set("fill", "white");
    let clip_path = element::ClipPath::new().set("id", params.id).add(rect);

    let img = element::Image::new()
        .set("x", params.x - params.x_padding)
        .set("y", params.y - params.y_padding)
        .set("width", params.width + params.x_padding * 2.0)
        .set("href", format!("data:image/jpeg;base64,{}", params.image))
        .set("clip-path", format!("url(#{})", params.id));

    document.append(img);
    definitions.append(clip_path);
}

#[derive(Debug, Clone, Copy)]
struct BoxParams<'a> {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color_id: &'a [Color],
    rotated: bool,
}

fn color_id_box(
    document: &mut element::SVG,
    definitions: &mut element::Definitions,
    params: BoxParams,
    gradient_id: &str,
) {
    let mut rect = Rectangle::new()
        .set("x", params.x)
        .set("y", params.y)
        .set("width", params.width)
        .set("height", params.height);

    if params.color_id.len() <= 1 {
        let color = params.color_id.first().map_or(COLORLESS_COLOR, |x| x.as_str());
        rect.assign("fill", color);
    } else {
        assert!(params.color_id.len() > 1);
        let mut gradient = element::LinearGradient::new().set("id", gradient_id);

        if params.rotated {
            // Changes gradient direction
            gradient.assign("x1", 0);
            gradient.assign("x2", 0);
            gradient.assign("y1", 1);
            gradient.assign("y2", 0);
        }

        let len = (params.color_id.len() - 1) as f64;
        for (i, color) in params.color_id.iter().enumerate() {
            gradient.append(
                element::Stop::new()
                    .set("offset", format!("{}%", 100.0 * i as f64 / len))
                    .set("stop-color", color.as_str()),
            );
        }
        definitions.append(gradient);

        rect.assign("fill", format!("url(#{})", gradient_id));
    }

    document.append(rect);
}

struct SVGConfig {
    // Adjacent SVG elements may display a gap, so we add `epsilon` overlap in some
    // places
    epsilon: f64,

    color_width: f64,
    image_height_x: f64,
    image_width_y: f64,
    box_width: f64,
    rotation: f64,
}

impl Default for SVGConfig {
    fn default() -> Self {
        Self {
            epsilon: 0.01,
            color_width: 0.2,
            image_height_x: 9.0,
            image_width_y: 8.5,
            box_width: 1.4,
            rotation: 37.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Labels<'a> {
    pub values: &'a [Partner],
    pub order: &'a [usize],
    pub images: &'a [String],
}

pub fn create_svg(
    x_labels: Labels<'_>,
    y_labels: Labels<'_>,
    decks: &IndexMap<(String, String), u64>,
) -> impl svg::Node {
    let config = SVGConfig::default();

    create_svg_from_config(x_labels, y_labels, decks, &config)
}

fn label_with_image(
    definitions: &mut element::Definitions,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    id: &str,
    label: &str,
    image: &str,
    rotation: f64,
) -> element::Group {
    let mut label_g = element::Group::new();

    let scale_factor = f64::cos(rotation.to_radians());

    let push_up = f64::sin(rotation.to_radians());

    let push_out = 1.0 / scale_factor;

    let params = ImageParams {
        x: 0.0,
        y: 0.0,
        height: height * scale_factor,
        width,
        x_padding: 0.5,
        y_padding: 0.5,
        id,
        image,
    };

    add_image(&mut label_g, definitions, params);

    let text = outlined_text(
        "start",
        0.2 + push_out * 0.35,
        height * scale_factor * (0.5 - (push_up * 0.1)),
        label,
        0.55,
        "Times New Roman",
    );
    text.add(&mut label_g);

    label_g.assign("transform", format!("translate({}, {}) rotate({})", x, y, -90.0 + rotation));
    label_g
}

fn create_svg_from_config(
    x_labels: Labels<'_>,
    y_labels: Labels<'_>,
    decks: &IndexMap<(String, String), u64>,
    config: &SVGConfig,
) -> element::SVG {
    let scale = 30.0;
    let x_len = x_labels.order.len();
    let y_len = y_labels.order.len();
    let mut min = u64::MAX;
    let mut max = u64::MIN;
    for &x in decks.values() {
        if x < min {
            min = x;
        }
        if x > max {
            max = x;
        }
    }

    let margin_right = config.rotation.to_radians().sin()
        * (config.image_height_x - config.box_width * config.rotation.to_radians().sin());
    let margin_top = config.rotation.to_radians().cos() * config.image_height_x
        - config.box_width * config.rotation.to_radians().sin();

    let width =
        x_len as f64 * config.box_width + config.image_width_y + config.color_width + margin_right;
    let height = y_len as f64 + margin_top + config.color_width;

    let mut document = Document::new()
        .set("width", scale * width)
        .set("height", scale * height)
        .set("viewBox", (0, 0, width, height));

    // Add background
    document.append(
        element::Rectangle::new()
            .set("x", 0)
            .set("y", 0)
            .set("width", width)
            .set("height", height)
            .set("fill", "rgb(34, 2, 45)"),
    );

    let mut definitions = element::Definitions::new();

    // Add images
    for (i, &label_i) in x_labels.order.iter().enumerate() {
        let id = format!("x-{i}");

        let x = i as f64 * config.box_width + config.image_width_y + config.color_width;
        let y = margin_top;

        let label = label_with_image(
            &mut definitions,
            x,
            y,
            config.image_height_x + config.epsilon,
            config.box_width + config.epsilon,
            &id,
            &x_labels.values[label_i].name,
            &x_labels.images[label_i],
            config.rotation,
        );

        document.append(label);

        let params = BoxParams {
            x,
            y,
            width: config.box_width + config.epsilon,
            height: config.color_width + config.epsilon,
            color_id: &x_labels.values[label_i].color_id,
            rotated: false,
        };

        let gradient_id = format!("x-gradient-{}", i);
        color_id_box(&mut document, &mut definitions, params, &gradient_id);
    }

    let mut y_paddings: Vec<f64> = Vec::with_capacity(y_labels.values.len());

    for y in y_labels.values {
        let mut out = 2.8;
        if let Some(y_offset_offset) = y.offset_y {
            out -= y_offset_offset;
        }
        y_paddings.push(out);
    }

    for (j, &label_j) in y_labels.order.iter().enumerate() {
        let id = format!("y-{j}");

        let inner_y = margin_top + config.color_width + j as f64;

        let params = ImageParams {
            x: 0.0,
            y: margin_top + config.color_width + j as f64,
            height: 1.0 + config.epsilon,
            width: config.image_width_y + config.epsilon,
            x_padding: 0.5,
            y_padding: y_paddings[label_j],
            id: &id,
            image: &y_labels.images[label_j],
        };

        add_image(&mut document, &mut definitions, params);

        // Add color
        let params = BoxParams {
            x: config.image_width_y,
            y: inner_y,
            width: config.color_width + config.epsilon,
            height: 1.0 + config.epsilon,
            color_id: &y_labels.values[label_j].color_id,
            rotated: true,
        };

        let gradient_id = format!("y-gradient-{}", j);
        color_id_box(&mut document, &mut definitions, params, &gradient_id);

        let text = outlined_text(
            "end",
            config.image_width_y - 0.2,
            margin_top + config.color_width + j as f64 + 0.5,
            &y_labels.values[label_j].name,
            0.55,
            "Times New Roman",
        );
        text.add(&mut document);
    }

    document.append(definitions);

    let mut texts: Vec<TextOutlined> = Vec::new();

    // Draw boxes
    for (j, &label_j) in y_labels.order.iter().enumerate() {
        let y = &y_labels.values[label_j];
        for (i, &label_i) in x_labels.order.iter().enumerate() {
            let x = &x_labels.values[label_i];
            let x_pos = i as f64 * config.box_width + config.image_width_y + config.color_width;
            let y_pos = j as f64 + margin_top + config.color_width;
            if x.name == y.name {
                let color = colorous::Color { r: 34, g: 2, b: 45 };
                add_color_cell(&mut document, config, color, x_pos, y_pos);
            } else {
                let value = decks
                    .get(&(x.name.clone(), y.name.clone()))
                    .or_else(|| decks.get(&(y.name.clone(), x.name.clone())))
                    .unwrap_or(&0);
                let color = get_color(min, max, *value);
                add_color_cell(&mut document, config, color, x_pos, y_pos);

                let text = outlined_text(
                    "middle",
                    x_pos + 0.5 * config.box_width,
                    y_pos + 0.5,
                    &value.to_string(),
                    0.5,
                    "sans-serif",
                );
                texts.push(text);
            }
        }
    }

    for text in texts {
        text.add(&mut document);
    }

    document.append(create_info());

    document
}

const EDHREC_IMAGE: &[u8] = include_bytes!("../static/edhrec.png");

fn create_info() -> element::Group {
    let mut out = element::Group::new();
    let mut image = String::new();
    BASE64_STANDARD.encode_string(EDHREC_IMAGE, &mut image);

    let edhrec_image = element::Image::new()
        .set("x", 2.7)
        .set("y", 4.3)
        .set("width", 4.4)
        .set("href", format!("data:image/jpeg;base64,{}", image));

    outlined_text("start", 0.5, 1.0, "Number of Commander decks by", 0.8, "Times New Roman")
        .add(&mut out);

    outlined_text("start", 0.5, 2.2, "partner pairs. Data was collected", 0.8, "Times New Roman")
        .add(&mut out);

    outlined_text("start", 0.5, 3.4, "2026-08-26 from:", 0.8, "Times New Roman").add(&mut out);

    out.append(edhrec_image);

    out
}

fn get_color(min: u64, max: u64, value: u64) -> colorous::Color {
    if value != 0 {
        let ratio = (value - min) as f64 / (max - min) as f64;
        let scaled = 1.0 - (1.0 - ratio).powi(15);
        VIRIDIS.eval_continuous(scaled)
    } else {
        MISSING_COLOR
    }
}

fn add_color_cell(
    document: &mut element::SVG,
    config: &SVGConfig,
    color: colorous::Color,
    x_pos: f64,
    y_pos: f64,
) {
    let r = Rectangle::new()
        .set("x", x_pos)
        .set("y", y_pos)
        .set("width", config.box_width * (1.0 + config.epsilon))
        .set("height", 1.0 + config.epsilon)
        .set("fill", format!("rgb({}, {}, {})", color.r, color.g, color.b));

    document.append(r);
}

struct TextOutlined {
    text: element::Text,

    // We add two outlines to prevent gaps between the outline
    outline: (element::Text, element::Text),
}

impl TextOutlined {
    fn add(self, svg: &mut impl svg::Node) {
        svg.append(self.outline.0);
        svg.append(self.outline.1);
        svg.append(self.text);
    }
}

fn outlined_text(
    text_anchor: &str,
    x: f64,
    y: f64,
    text: &str,
    font_size: f64,
    font_family: &str,
) -> TextOutlined {
    let text = element::Text::new(text)
        .set("x", x)
        .set("y", y)
        .set("text-anchor", text_anchor)
        .set("dominant-baseline", "central")
        .set("font-size", font_size)
        .set("style", format!("font-family:{font_family};"));

    let text_inner = text.clone().set("fill", "white");

    TextOutlined {
        text: text_inner,
        outline: (text_outline(font_family, 0.2, &text), text_outline(font_family, 0.1, &text)),
    }
}

fn text_outline(font_family: &str, stroke_width: f64, text: &element::Text) -> element::Text {
    let style = format!(
        concat!(
            "font-family:{};",
            "paint-order: stroke fill;",
            "stroke: #000000;",
            "stroke-width: {};",
            "stroke-linecap: butt;",
            "stroke-linejoin: round;",
            "fill-rule: nonzero;",
            "user-select: none;",
        ),
        font_family, stroke_width
    );
    text.clone().set("fill", "black").set("style", style)
}
