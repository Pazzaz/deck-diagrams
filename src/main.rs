use std::{
    collections::HashMap,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

use base64::prelude::*;
use draw::create_svg;
use indexmap::IndexMap;
use serde::Serialize as _;
use web::{WebParameters, update_data};

use crate::{
    draw::Labels,
    partner_data::{Data, Partner},
    render::svg_to_png,
};

mod color;
mod draw;
mod partner_data;
mod render;
mod web;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "deck-diagrams")]
#[command(about = "CLI to download statistics from EDHREC and generate diagrams", long_about = None)]
struct Cli {
    #[arg(help = "Path to JSON file containing statistics")]
    data: OsString,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Download(DownloadArgs),

    #[command(about = "Render SVG diagram", long_about = None)]
    #[command(arg_required_else_help = true)]
    Render {
        output: OsString,
    },
}

#[derive(Debug, Args)]
#[command(about = "Download statistics", long_about = Some("Download partner/deck data. Provide at least one parameter to start downloading."))]
#[command(arg_required_else_help = true)]
// Requires at least one of them to be active
#[group(required = true)]
struct DownloadArgs {
    #[arg(help = "Download number of decks")]
    #[arg(short = 'd', long)]
    decks: bool,

    #[arg(help = "Download partners' color identities")]
    #[arg(long)]
    ids: bool,

    #[arg(help = "Download partners' images")]
    #[arg(long)]
    img: bool,
}

fn main() {
    let args = Cli::try_parse().unwrap_or_else(|x| x.exit());

    let data_file = PathBuf::from_str(args.data.to_str().unwrap()).unwrap();

    let data_folder = data_file.parent().unwrap();

    let f = fs::File::open(&data_file).unwrap();
    let mut data: Data = serde_json::from_reader(f).unwrap();

    match args.command {
        Commands::Download(DownloadArgs { decks, ids, img }) => {
            let to_download = WebParameters::new(decks, ids);

            if to_download.any() {
                let image_path = data_folder.join("images");
                let images = if img { Some(image_path.as_path()) } else { None };
                update_data(&mut data, to_download, images);
            }

            let mut out = fs::File::create(data_file).unwrap();

            let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
            let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
            data.serialize(&mut ser).unwrap();
        }
        Commands::Render { output } => {
            let output_path = PathBuf::from_str(output.to_str().unwrap()).unwrap();
            let svg = match data {
                Data::Single(data_single) => {
                    let images = get_images(&data_folder.join("images"), &data_single.values);
                    let order = get_order(&data_single.values, &data_single.decks, Counting::Both);
                    let labels =
                        Labels { values: &data_single.values, order: &order, images: &images };

                    create_svg(labels, labels, &data_single.decks)
                }
                Data::Double(data_double) => {
                    let x_images = get_images(&data_folder.join("images"), &data_double.x_values);
                    let y_images = get_images(&data_folder.join("images"), &data_double.y_values);

                    let x_order = get_order(&data_double.x_values, &data_double.decks, Counting::X);
                    let y_order = get_order(&data_double.y_values, &data_double.decks, Counting::Y);

                    let x_labels = Labels {
                        values: &data_double.x_values,
                        order: &x_order,
                        images: &x_images,
                    };

                    let y_labels = Labels {
                        values: &data_double.y_values,
                        order: &y_order,
                        images: &y_images,
                    };

                    create_svg(x_labels, y_labels, &data_double.decks)
                }
            };

            svg_to_png(&svg, &output_path);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Counting {
    X,
    Y,
    Both,
}

fn get_order(
    values: &[Partner],
    decks: &IndexMap<(String, String), u64>,
    counting: Counting,
) -> Vec<usize> {
    let len = values.len();

    let mut order: Vec<usize> = (0..len).collect();

    let mut total_partner = vec![0; len];

    let name_to: HashMap<String, usize> = {
        let mut out = HashMap::new();
        for (i, name) in values.iter().map(|x| &x.name).enumerate() {
            out.insert(name.clone(), i);
        }
        out
    };

    for ((x, y), v) in decks {
        match counting {
            Counting::X => total_partner[name_to[x]] += v,
            Counting::Y => total_partner[name_to[y]] += v,
            Counting::Both => {
                total_partner[name_to[x]] += v;
                total_partner[name_to[y]] += v;
            }
        }
    }

    order.sort_by_key(|&i| std::cmp::Reverse((total_partner[i], i)));

    order
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
