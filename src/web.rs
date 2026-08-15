use std::fmt::Display;

use crate::{Color, Data};

use regex::Regex;
use reqwest::StatusCode;

struct ParseData {
    deck_count: u64,
    id_0: Vec<Color>,
    id_1: Vec<Color>,
}

impl ParseData {
    fn diff(&self, other: (u64, &[Color], &[Color])) -> WebParameters {
        WebParameters {
            deck_count: self.deck_count != other.0,
            color_0: &self.id_0[..] != other.1,
            color_1: &self.id_1[..] != other.2,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WebParameters {
    deck_count: bool,
    color_0: bool,
    color_1: bool,
}

impl WebParameters {
    pub const fn new(deck_count: bool, colors: bool) -> Self {
        Self {
            deck_count,
            color_0: colors,
            color_1: colors,
        }
    }
    pub const fn any(self) -> bool {
        self.deck_count || self.color_0 || self.color_1
    }

    const fn and(self, other: Self) -> Self {
        Self {
            deck_count: self.deck_count && other.deck_count,
            color_0: self.color_0 && other.color_0,
            color_1: self.color_1 && other.color_1,
        }
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

pub fn update_data(data: &mut Data, to_download: WebParameters) {
    debug_assert!(to_download.any());
    let client = reqwest::blocking::Client::new();

    for x in &mut data.x_values {
        let a = slugify(&x.name);
        for y in &mut data.y_values {
            let b = slugify(&y.name);
            let downloaded = match download_data(&a, &b, &client) {
                Ok(x) => x,
                Err(x) => {
                    eprintln!("{x}");
                    continue;
                }
            };
            let old_c = *data
                .decks
                .get(&(x.name.clone(), y.name.clone()))
                .unwrap_or(&0);
            let diff = downloaded.diff((old_c, &x.id, &y.id));
            let to_update = diff.and(to_download);
            if to_update.any() {
                let url = format!("https://edhrec.com/commanders/{a}-{b}");
                println!("Updated {url}");
                if to_update.deck_count {
                    println!("count: {} -> {}", old_c, downloaded.deck_count);
                    data.decks
                        .insert((x.name.clone(), y.name.clone()), downloaded.deck_count);
                }
                if to_update.color_0 {
                    println!("color of first: {:?} -> {:?}", x.id, downloaded.id_0);
                    x.id = downloaded.id_0;
                }

                if to_update.color_1 {
                    println!("color of second: {:?} -> {:?}", y.id, downloaded.id_1);
                    y.id = downloaded.id_1;
                }
            }
        }
    }
}

fn download_data(
    slug1: &str,
    slug2: &str,
    client: &reqwest::blocking::Client,
) -> Result<ParseData, WebError> {
    let url = format!("https://edhrec.com/commanders/{slug1}-{slug2}");
    let mut resp = client.get(&url).send().map_err(WebError::Request)?;

    // If we don't reach a valid url, we'll try switching the partner order
    let switched = resp.status() != StatusCode::OK;

    if switched {
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
        let mut data = parse_json(&v).ok_or(WebError::ParseFail)?;
        if switched {
            (data.id_0, data.id_1) = (data.id_1, data.id_0);
        }
        Ok(data)
    } else {
        Err(WebError::MissingData)
    }
}

fn parse_json(v: &serde_json::Value) -> Option<ParseData> {
    let json_dict = &v.pointer("/props/pageProps/data/container/json_dict")?;
    let cards = json_dict.pointer("/card/cards")?.as_array()?;
    if let [card_0, card_1] = &cards[..] {
        let id_0 = parse_color(card_0.get("color_identity")?)?;
        let id_1 = parse_color(card_1.get("color_identity")?)?;
        let deck_count = json_dict.pointer("/card/num_decks")?.as_u64()?;
        let out = ParseData {
            deck_count,
            id_0,
            id_1,
        };
        Some(out)
    } else {
        None
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .replace(char::is_whitespace, "-")
        .replace([',', '\''], "")
}

fn parse_color(v: &serde_json::Value) -> Option<Vec<Color>> {
    v.as_array()?
        .iter()
        .map(|x| x.as_str().and_then(|y| y.parse().ok()))
        .collect::<Option<Vec<Color>>>()
}
