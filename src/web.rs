use std::{collections::HashMap, fmt::Display};

use crate::{Color, Data};

use regex::Regex;
use reqwest::StatusCode;

struct ParseData {
    deck_count: u64,
    id_0: Vec<Color>,
    id_1: Vec<Color>,
}

#[derive(Debug, Clone, Copy)]
struct ParseDataDiff {
    deck_count: bool,
    color_0: bool,
    color_1: bool,
}

impl ParseData {
    fn diff(&self, other: (u64, &[Color], &[Color])) -> ParseDataDiff {
        ParseDataDiff {
            deck_count: self.deck_count != other.0,
            color_0: &self.id_0[..] != other.1,
            color_1: &self.id_1[..] != other.2,
        }
    }
}

impl ParseDataDiff {
    fn any(&self) -> bool {
        self.deck_count || self.color_0 || self.color_1
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
            WebError::Request(error) => f.write_fmt(format_args!("Error: {}", error)),
            WebError::BadStatus => f.write_str("Could not reach URL"),
            WebError::MissingData => f.write_str("Could not find data at URL"),
            WebError::ParseFail => f.write_str("Could not parse data"),
        }
    }
}

pub fn update_data(data: &mut Data) {
    let client = reqwest::blocking::Client::new();

    for x in data.x_values.iter_mut() {
        for y in data.y_values.iter_mut() {
            let downloaded = match download_data(&x.name, &y.name, &client) {
                Ok(x) => x,
                Err(x) => {
                    eprintln!("{}", x);
                    continue;
                }
            };
            let old_c = *data
                .companions
                .get(&(x.name.clone(), y.name.clone()))
                .unwrap_or(&0);
            let diff = downloaded.diff((old_c, &x.id, &y.id));
            if diff.any() {
                let a = slugify(&x.name);
                let b = slugify(&y.name);
                let url = format!("https://edhrec.com/commanders/{}-{}", a, b);
                println!("Updated {}", url);
                if diff.deck_count {
                    println!("count: {} -> {}", old_c, downloaded.deck_count);
                    data.companions
                        .insert((x.name.clone(), y.name.clone()), downloaded.deck_count);
                }
                if diff.color_0 {
                    println!("color of first: {:?} -> {:?}", &x.id, &downloaded.id_0);
                    x.id = downloaded.id_0;
                }
                if diff.color_1 {
                    println!("color of second: {:?} -> {:?}", &y.id, &downloaded.id_1);
                    y.id = downloaded.id_1;
                }
            }
        }
    }
}

fn download_data(
    partner1: &str,
    partner2: &str,
    client: &reqwest::blocking::Client,
) -> Result<ParseData, WebError> {
    let a = slugify(partner1);
    let b = slugify(partner2);
    let url = format!("https://edhrec.com/commanders/{}-{}", a, b);
    let mut resp = client.get(&url).send().map_err(WebError::Request)?;

    let mut switched = false;
    if resp.status() != StatusCode::OK {
        // Try switching the two partners in the URL
        let url = format!("https://edhrec.com/commanders/{}-{}", b, a);
        resp = client.get(&url).send().map_err(WebError::Request)?;
        if resp.status() != StatusCode::OK {
            return Err(WebError::BadStatus);
        }

        switched = true;
    }

    let content = resp.text().map_err(WebError::Request)?;

    // Extract
    let re =
        Regex::new(r#"<script id="__NEXT_DATA__" type="application/json">(.+)</script>"#).unwrap();

    if let Some(capture) = re.captures(&content) {
        let block = capture.get(1).unwrap().as_str();
        let v: serde_json::Value = serde_json::from_str(block).unwrap();
        let json_dict = &v["props"]["pageProps"]["data"]["container"]["json_dict"];
        let cards = json_dict["card"]["cards"].as_array().unwrap();
        assert!(cards.len() == 2);
        let mut id_0 = parse_color(&cards[0]["color_identity"]).ok_or(WebError::ParseFail)?;
        let mut id_1 = parse_color(&cards[1]["color_identity"]).ok_or(WebError::ParseFail)?;
        if switched {
            (id_0, id_1) = (id_1, id_0);
        }
        let deck_count = json_dict["card"]["num_decks"].as_u64().unwrap();
        let out = ParseData {
            deck_count,
            id_0,
            id_1,
        };
        Ok(out)
    } else {
        Err(WebError::MissingData)
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .replace(char::is_whitespace, "-")
        .replace(',', "")
        .replace('\'', "")
}

fn parse_color(v: &serde_json::Value) -> Option<Vec<Color>> {
    v.as_array()?
        .iter()
        .map(|x| x.as_str().and_then(|y| y.parse().ok()))
        .collect::<Option<Vec<Color>>>()
}
