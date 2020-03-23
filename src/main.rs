#![feature(async_closure)]
#[macro_use]
extern crate serde_json;

use std::collections::HashMap;
use std::env;

use async_socks5::Auth;
use futures::StreamExt;
use handlebars::Handlebars;
use hyper::Uri;
use hyper_socks2::SocksConnector;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use telegram_bot::connector::hyper::HyperConnector;
use telegram_bot::*;

#[derive(Deserialize, Serialize)]
struct LatestResult {
    confirmed: i64,
    deaths: i64,
    recovered: i64,
}

#[derive(Deserialize, Serialize)]
struct CovidCountryResult {
    latest: LatestResult,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");

    let api = if let Ok(adr) = env::var("SOCKS_PROXY") {
        Api::with_connector(
            token,
            Box::new(HyperConnector::new(
                hyper::Client::builder().build(
                    SocksConnector {
                        proxy_addr: Uri::from_str(dbg!(adr.as_str())).unwrap(), // scheme is required by HttpConnector
                        auth: Some(Auth::new(
                            env::var("SOCKS_PROXY_USERNAME").unwrap(),
                            env::var("SOCKS_PROXY_PASSWORD").unwrap(),
                        )),
                        connector: hyper_tls::HttpsConnector::new(),
                    }
                    .with_tls()
                    .unwrap(),
                ),
            )),
        )
    } else {
        Api::with_connector(
            token,
            Box::new(HyperConnector::new(
                hyper::Client::builder().build(hyper_tls::HttpsConnector::new()),
            )),
        )
    };

    let mut reg = Handlebars::new();
    reg.register_template_string(
        "UA",
        String::from("У хохлов, по последним данным: {{ confirmed }} всего, {{ deaths }} умерло, {{ recovered }} вылечилось"),
    )?;
    reg.register_template_string(
        "RU",
        String::from("У москалів, за останніми даними: {{ confirmed }} все, {{ deaths }} померло, {{ recovered }} вилікувалося"),
    )?;

    let mut country_to_country_code: HashMap<&str, String> = HashMap::new();
    country_to_country_code.insert("хохлов", String::from("UA"));
    country_to_country_code.insert("москалів", String::from("RU"));

    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        let update = update?;
        if let UpdateKind::Message(message) = update.kind {
            if let MessageKind::Text { ref data, .. } = message.kind {
                if data.starts_with("/шо_там_у_") {
                    let country = data.chars().skip(10).collect::<String>();
                    if let Some(county_code) = country_to_country_code.get(country.as_str()) {
                        async {
                            let county_code = county_code.clone();
                            let response = get_covid_stats(county_code.as_str())
                                .await
                                .map(|stats| {
                                    reg.render(county_code.as_str(), &json!(stats))
                                        .map(|msg| message.text_reply(msg))
                                        .unwrap()
                                })
                                .unwrap();
                            api.send(response).await
                        }
                        .await?;
                    }
                };
            }
        }
    }
    Ok(())
}

async fn get_covid_stats(country: &str) -> Result<LatestResult, Box<dyn std::error::Error>> {
    let country_results: CovidCountryResult = reqwest::get(
        format!(
            "https://coronavirus-tracker-api.herokuapp.com/v2/locations?country_code={}",
            country
        )
        .as_str(),
    )
    .await?
    .json::<CovidCountryResult>()
    .await?;

    Ok(country_results.latest)
}
