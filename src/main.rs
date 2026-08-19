use std::{
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

use base64::prelude::*;
use draw::create_svg;
use serde::Serialize as _;
use web::{WebParameters, update_data};

use crate::partner_data::{Data, Partner};

mod color;
mod draw;
mod partner_data;
mod web;

fn main() {
    let data_folder = PathBuf::from_str("./data/partner").unwrap();
    let download_counts: bool = false;
    let download_color_ids: bool = false;
    let download_images: bool = false;
    let save_data: bool = false;

    let f = fs::File::open(data_folder.join("data.json")).unwrap();
    let mut data: Data = serde_json::from_reader(f).unwrap();

    let to_download = WebParameters::new(download_counts, download_color_ids);

    if to_download.any() {
        let image_path = data_folder.join("images");
        let images = if download_images { Some(image_path.as_path()) } else { None };
        update_data(&mut data, to_download, images);
    }

    if save_data {
        let mut out = fs::File::create(data_folder.join("data.json")).unwrap();

        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
        data.serialize(&mut ser).unwrap();
    }

    let svg = match data {
        Data::Single(data_single) => {
            let images = get_images(&data_folder.join("images"), &data_single.values);
            let values = &data_single.values;

            create_svg(values, values, &data_single.decks, &images, &images, true, false)
        }
        Data::Double(data_double) => {
            let x_images = get_images(&data_folder.join("images"), &data_double.x_values);
            let y_images = get_images(&data_folder.join("images"), &data_double.y_values);
            create_svg(
                &data_double.x_values,
                &data_double.y_values,
                &data_double.decks,
                &x_images,
                &y_images,
                false,
                true,
            )
        }
    };

    svg::save("./out7.svg", &svg).unwrap();
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
