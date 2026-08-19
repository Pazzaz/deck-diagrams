use std::{fmt::Display, fs, io::Write, mem, path::Path};

use regex::Regex;
use reqwest::StatusCode;

use crate::{
    color::Color,
    partner_data::{Data, DataDouble, DataSingle},
};

struct ParseData {
    deck_count: u64,
    color_id_0: Vec<Color>,
    color_id_1: Vec<Color>,
    image_url_0: String,
    image_url_1: String,
}

impl ParseData {
    const fn switch(&mut self) {
        mem::swap(&mut self.color_id_0, &mut self.color_id_1);
        mem::swap(&mut self.image_url_0, &mut self.image_url_1);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WebParameters {
    deck_count: bool,
    color_id_0: bool,
    color_id_1: bool,
}

impl WebParameters {
    pub const fn new(deck_count: bool, color_id: bool) -> Self {
        Self { deck_count, color_id_0: color_id, color_id_1: color_id }
    }

    pub const fn any(self) -> bool {
        self.deck_count || self.color_id_0 || self.color_id_1
    }
}

#[derive(Debug)]
enum WebError {
    Request(reqwest::Error),
    BadStatus,
    MissingData,
    ParseFail,
}

impl Display for WebError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(error) => f.write_fmt(format_args!("Error: {error}")),
            Self::BadStatus => f.write_str("Could not reach URL"),
            Self::MissingData => f.write_str("Could not find data at URL"),
            Self::ParseFail => f.write_str("Could not parse data"),
        }
    }
}

pub fn update_data(data: &mut Data, to_download: WebParameters, image_folder: Option<&Path>) {
    match data {
        Data::Single(data_single) => update_data_single(data_single, to_download, image_folder),
        Data::Double(data_double) => update_data_double(data_double, to_download, image_folder),
    }
}

pub fn update_data_double(
    data: &mut DataDouble,
    to_download: WebParameters,
    image_folder: Option<&Path>,
) {
    debug_assert!(to_download.any());
    let client = reqwest::blocking::Client::new();

    // When downloading images, we only download the first run through the
    // `y_values`.
    let mut new_y = true;
    for x in &mut data.x_values {
        // Similarly for the `x_values`, we don't want to download their image every
        // execution of the inner loop.
        let mut new_x = true;
        for y in &mut data.y_values {
            if !(to_download.deck_count || new_x || new_y) {
                break;
            }
            let deck_count = if to_download.deck_count {
                Some(*data.decks.get(&(x.name.clone(), y.name.clone())).unwrap_or(&0))
            } else {
                None
            };
            let x_color_id =
                if new_x && to_download.color_id_0 { Some(&x.color_id[..]) } else { None };
            let y_color_id =
                if new_y && to_download.color_id_1 { Some(&y.color_id[..]) } else { None };

            let entry = EntryDownload {
                x_name: &x.name,
                y_name: &y.name,
                x_image: new_x,
                y_image: new_y,
                deck_count,
                x_color_id,
                y_color_id,
            };

            let downloaded = download_entry(entry, image_folder, &client);

            if let Some(new_deck_count) = downloaded.deck_count {
                data.decks.insert((x.name.clone(), y.name.clone()), new_deck_count);
            }

            if let Some(new_x_color_id) = downloaded.x_color_id {
                x.color_id = new_x_color_id;
            }

            if let Some(new_y_color_id) = downloaded.y_color_id {
                y.color_id = new_y_color_id;
            }

            new_x = false;
        }
        new_y = false;
    }
}

struct EntryDownload<'a> {
    x_name: &'a str,
    y_name: &'a str,
    x_image: bool,
    y_image: bool,
    deck_count: Option<u64>,
    x_color_id: Option<&'a [Color]>,
    y_color_id: Option<&'a [Color]>,
}

impl<'a> EntryDownload<'a> {
    const fn any(&self) -> bool {
        self.x_image
            || self.y_image
            || self.deck_count.is_some()
            || self.x_color_id.is_some()
            || self.y_color_id.is_some()
    }
}

#[derive(Debug, Default)]
struct Update {
    deck_count: Option<u64>,
    x_color_id: Option<Vec<Color>>,
    y_color_id: Option<Vec<Color>>,
}

// Downloads the specified info, including saving images to `image_folder`
fn download_entry(
    entry: EntryDownload,
    image_folder: Option<&Path>,
    client: &reqwest::blocking::Client,
) -> Update {
    debug_assert!(entry.any());

    println!("Downloading: {} | {}", entry.x_name, entry.y_name);

    let mut update = Update::default();

    let a = slugify(entry.x_name);
    let b = slugify(entry.y_name);
    let downloaded = match download_data(entry.x_name, entry.y_name, &a, &b, client) {
        Ok(x) => x,
        Err(x) => {
            eprintln!("{x}");
            return update;
        }
    };

    if let Some(image_folder) = image_folder {
        if entry.x_image {
            let mut f =
                fs::File::create(image_folder.join(format!("{}.jpg", entry.x_name))).unwrap();
            let image_0 = download_image(client, &downloaded.image_url_0).unwrap();
            f.write_all(&image_0).unwrap();
        }

        if entry.y_image {
            let mut f =
                fs::File::create(image_folder.join(format!("{}.jpg", entry.y_name))).unwrap();
            let image_1 = download_image(client, &downloaded.image_url_1).unwrap();
            f.write_all(&image_1).unwrap();
        }
    }
    if let Some(old_deck_count) = entry.deck_count
        && old_deck_count != downloaded.deck_count
    {
        update.deck_count = Some(downloaded.deck_count);
    }
    if let Some(old_x_color_id) = entry.x_color_id
        && old_x_color_id != downloaded.color_id_0
    {
        update.x_color_id = Some(downloaded.color_id_0);
    }
    if let Some(old_y_color_id) = entry.y_color_id
        && old_y_color_id != downloaded.color_id_1
    {
        update.y_color_id = Some(downloaded.color_id_1);
    }
    update
}

pub fn update_data_single(
    data: &mut DataSingle,
    to_download: WebParameters,
    image_folder: Option<&Path>,
) {
    debug_assert!(to_download.any());
    let client = reqwest::blocking::Client::new();

    for j in 1..data.values.len() {
        let (lower, upper) = data.values.split_at_mut(j);
        let y = &mut upper[0];

        // When downloading images, we only download the first run through
        let mut new = true;
        for x in lower {
            if !(to_download.deck_count || new) {
                break;
            }
            let deck_count = if to_download.deck_count {
                Some(*data.decks.get(&(x.name.clone(), y.name.clone())).unwrap_or(&0))
            } else {
                None
            };
            let x_color_id =
                if new && to_download.color_id_0 { Some(&x.color_id[..]) } else { None };

            let entry = EntryDownload {
                x_name: &x.name,
                y_name: &y.name,
                x_image: new,
                y_image: false,
                deck_count,
                x_color_id,
                y_color_id: None,
            };

            let downloaded = download_entry(entry, image_folder, &client);

            if let Some(new_deck_count) = downloaded.deck_count {
                data.decks.insert((x.name.clone(), y.name.clone()), new_deck_count);
            }

            if let Some(new_x_color_id) = downloaded.x_color_id {
                x.color_id = new_x_color_id;
            }

            new = false;
        }
    }
}

fn download_image(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, WebError> {
    let resp = client.get(url).send().map_err(WebError::Request)?;
    if resp.status() != StatusCode::OK {
        return Err(WebError::BadStatus);
    }

    let content = resp.bytes().map_err(WebError::Request)?;

    Ok(content.to_vec())
}

fn download_data(
    x_name: &str,
    y_name: &str,
    slug1: &str,
    slug2: &str,
    client: &reqwest::blocking::Client,
) -> Result<ParseData, WebError> {
    let url = format!("https://edhrec.com/commanders/{slug1}-{slug2}");
    let mut resp = client.get(&url).send().map_err(WebError::Request)?;

    // If we don't reach a valid url, we'll try switching the partner order
    // May not be needed, seems like the site redirects sometimes
    if resp.status() != StatusCode::OK {
        let url = format!("https://edhrec.com/commanders/{slug2}-{slug1}");
        resp = client.get(&url).send().map_err(WebError::Request)?;

        // If it still didn't work, we return
        if resp.status() != StatusCode::OK {
            return Err(WebError::BadStatus);
        }
    }

    let content = resp.text().map_err(WebError::Request)?;

    // Extract
    let re =
        Regex::new(r#"<script id="__NEXT_DATA__" type="application/json">(.+)</script>"#).unwrap();

    if let Some(capture) = re.captures(&content).and_then(|x| x.get(1)) {
        let block = capture.as_str();
        let v: serde_json::Value = serde_json::from_str(block).map_err(|_| WebError::ParseFail)?;
        let data = parse_json(&v, x_name, y_name).ok_or(WebError::ParseFail)?;
        Ok(data)
    } else {
        Err(WebError::MissingData)
    }
}

fn parse_json(v: &serde_json::Value, x_name: &str, y_name: &str) -> Option<ParseData> {
    let card = &v.pointer("/props/pageProps/data/container/json_dict/card")?;
    let cards = card.pointer("/cards")?.as_array()?;
    if let [card_0, card_1] = &cards[..] {
        let name_0 = card_0.get("name")?.as_str()?;
        let name_1 = card_1.get("name")?.as_str()?;
        let switch = if (name_0, name_1) == (x_name, y_name) {
            false
        } else if (name_1, name_0) == (x_name, y_name) {
            true
        } else {
            return None;
        };
        let color_id_0 = parse_color(card_0.get("color_identity")?)?;
        let color_id_1 = parse_color(card_1.get("color_identity")?)?;
        let deck_count = card.pointer("/num_decks")?.as_u64()?;
        if let [card_images_0, card_images_1] = &card.get("image_uris")?.as_array()?[..] {
            let image_url_0 = card_images_0.get("art_crop")?.as_str()?.to_string();
            let image_url_1 = card_images_1.get("art_crop")?.as_str()?.to_string();
            let mut out =
                ParseData { deck_count, color_id_0, color_id_1, image_url_0, image_url_1 };
            if switch {
                out.switch();
            }
            Some(out)
        } else {
            None
        }
    } else {
        None
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase().replace(char::is_whitespace, "-").replace([',', '\''], "")
}

fn parse_color(v: &serde_json::Value) -> Option<Vec<Color>> {
    v.as_array()?.iter().map(|x| x.as_str().and_then(|y| y.parse().ok())).collect()
}
