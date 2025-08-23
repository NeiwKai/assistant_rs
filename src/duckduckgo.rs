use rand::{seq::SliceRandom, thread_rng, Rng};
use rand::prelude::IndexedRandom;
use reqwest::Client;
use scraper::{Html, Selector};
use std::{error::Error, time::Duration};
use tokio::time::sleep;

pub async fn duckduckgo_search(query: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let user_agents = vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 13_4_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.5 Safari/605.1.15",
        "Mozilla/5.0 (X11; Linux x86_64) Gecko/20100101 Firefox/117.0",
    ];

    let url = format!("https://duckduckgo.com/html/?q={}", query.replace(" ", "+"));
    let ua = user_agents.choose(&mut thread_rng()).unwrap();

    let client = Client::builder()
        .user_agent(*ua)
        .build()?;

    let resp = client
        .get(&url)
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await?;

    let body = resp.text().await?;

    // Simulate human delay
    let delay = thread_rng().gen_range(2..=5);
    sleep(Duration::from_secs(delay)).await;

    let doc = Html::parse_document(&body);
    let selector = Selector::parse("a.result__a").unwrap();

    let mut results = Vec::new();
    for element in doc.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if let Some(decoded_url) = extract_uddg_url(href) {
                if is_valid_target_url(&decoded_url) {
                    println!("Valid target URL: {}", decoded_url);
                    results.push(decoded_url);
                } else {
                    println!("Filtered out internal DuckDuckGo URL: {}", decoded_url);
                }
            }
        }
    }

    Ok(results)
}

use url::Url;
use urlencoding::decode;

fn is_valid_target_url(url_str: &str) -> bool {
    if let Ok(url) = Url::parse(url_str) {
        let host = url.host_str().unwrap_or("");
        if host.contains("duckduckgo.com") {
            return false;
        }
        return true;
    }
    false
}

/// Extracts and decodes the `uddg` parameter from a DuckDuckGo redirect URL
pub fn extract_uddg_url(raw_href: &str) -> Option<String> {
    // If the href starts with `//`, prepend scheme
    let full_url = if raw_href.starts_with("//") {
        format!("https:{}", raw_href)
    } else {
        raw_href.to_string()
    };

    // Parse URL and extract `uddg` query param
    let parsed = Url::parse(&full_url).ok()?;
    let query_pairs = parsed.query_pairs();

    for (key, value) in query_pairs {
        if key == "uddg" {
            // Decode percent-encoded URL
            return decode(&value).ok().map(|s| s.to_string());
        }
    }

    None
}



