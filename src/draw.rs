use colorous::VIRIDIS;
use svg::{
    Document, Node as _,
    node::element::{self, Rectangle},
};

use crate::{
    Data,
    color::{COLORLESS_COLOR, Color, MISSING_COLOR},
};

// Adjacent SVG elements may display a gap, so we add `EPSILON` overlap in some
// places
const EPSILON: f64 = 0.01;

const COLOR_WIDTH: f64 = 0.2;

const IMAGE_WIDTH_Y: f64 = 6.9;
const IMAGE_HEIGHT_X: f64 = 1.5;

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
    document: &mut element::SVG,
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
struct BoxParams {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn add_color_box(
    document: &mut element::SVG,
    definitions: &mut element::Definitions,
    params: BoxParams,
    id: &str,
    colors: &[Color],
    rotate_gradient: bool,
) {
    let mut rect = Rectangle::new()
        .set("x", params.x)
        .set("y", params.y)
        .set("width", params.width)
        .set("height", params.height);

    if colors.len() <= 1 {
        let color = colors.get(0).map_or(COLORLESS_COLOR, |x| x.as_str());
        rect.assign("fill", color);
    } else {
        assert!(colors.len() > 1);
        let mut gradient = element::LinearGradient::new().set("id", id);

        if rotate_gradient {
            gradient.assign("x1", 0);
            gradient.assign("x2", 0);
            gradient.assign("y1", 1);
            gradient.assign("y2", 0);
        }

        let len = (colors.len() - 1) as f64;
        for (i, color) in colors.iter().enumerate() {
            gradient.append(
                element::Stop::new()
                    .set("offset", format!("{}%", 100.0 * i as f64 / len))
                    .set("stop-color", color.to_string()),
            );
        }
        definitions.append(gradient);

        rect.assign("fill", format!("url(#{})", id));
    }

    document.append(rect);
}

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

    let width = x_len as f64 * box_width + IMAGE_WIDTH_Y + COLOR_WIDTH;
    let height = y_len as f64 + IMAGE_HEIGHT_X + COLOR_WIDTH;

    let mut document = Document::new()
        .set("width", scale * width)
        .set("height", scale * height)
        .set("viewBox", (0, 0, width, height));

    let mut definitions = element::Definitions::new();

    // Add images
    for (i, image) in x_images.iter().enumerate() {
        let id = format!("x-{i}");

        let x = i as f64 * box_width + IMAGE_WIDTH_Y + COLOR_WIDTH;

        let params = ImageParams {
            x,
            y: 0.0,
            height: IMAGE_HEIGHT_X + EPSILON,
            width: box_width,
            x_padding: 0.5,
            y_padding: 0.0,
            id: &id,
            image,
        };

        add_image(&mut document, &mut definitions, params);

        let info = &data.x_values[i];

        let params = BoxParams {
            x,
            y: IMAGE_HEIGHT_X,
            width: box_width + EPSILON,
            height: COLOR_WIDTH + EPSILON,
        };

        let id = format!("x-gradient-{}", i);
        add_color_box(&mut document, &mut definitions, params, &id, &info.id, false);
    }

    for (j, image) in y_images.iter().enumerate() {
        let info = &data.y_values[j];
        let id = format!("y-{j}");

        let inner_y = IMAGE_HEIGHT_X + COLOR_WIDTH + j as f64;
        let mut y_offset = 2.8;

        if let Some(y_offset_offset) = info.offset_y {
            y_offset -= y_offset_offset;
        }
        let params = ImageParams {
            x: 0.0,
            y: IMAGE_HEIGHT_X + COLOR_WIDTH + j as f64,
            height: 1.0 + EPSILON,
            width: IMAGE_WIDTH_Y + EPSILON,
            x_padding: 0.5,
            y_padding: y_offset,
            id: &id,
            image,
        };

        add_image(&mut document, &mut definitions, params);

        // Add color
        let params = BoxParams {
            x: IMAGE_WIDTH_Y,
            y: inner_y,
            width: COLOR_WIDTH + EPSILON,
            height: 1.0 + EPSILON,
        };

        let id = format!("y-gradient-{}", j);
        add_color_box(&mut document, &mut definitions, params, &id, &info.id, true);
    }

    document.append(definitions);

    for (j, label) in data.y_values.iter().enumerate() {
        let name = &label.name;
        outlined_text(
            &mut document,
            "end",
            IMAGE_WIDTH_Y - 0.2,
            IMAGE_HEIGHT_X + COLOR_WIDTH + j as f64 + 0.5,
            name,
            0.55,
            "serif",
        );
    }

    for (i, label) in data.x_values.iter().enumerate() {
        let name = match label.short.as_ref() {
            Some(x) => x,
            None => &label.name.chars().next().unwrap().to_string(),
        };
        outlined_text(
            &mut document,
            "middle",
            IMAGE_WIDTH_Y + COLOR_WIDTH + (0.5 + i as f64) * box_width,
            IMAGE_HEIGHT_X / 2.0,
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
                    (MISSING_COLOR, "0".to_string())
                };
            let x_pos = i as f64 * box_width + IMAGE_WIDTH_Y + COLOR_WIDTH;
            let y_pos = j as f64 + IMAGE_HEIGHT_X + COLOR_WIDTH;
            let r = Rectangle::new()
                .set("x", x_pos)
                .set("y", y_pos)
                .set("width", box_width * (1.0 + EPSILON))
                .set("height", 1.0 + EPSILON)
                .set("fill", format!("rgb({}, {}, {})", color.r, color.g, color.b));

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
