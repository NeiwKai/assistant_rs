use reqwest;
use scraper::{Html, Selector};

use crate::duckduckgo::duckduckgo_search;

pub async fn websearch(query: &str) -> String {
    let user_agents = vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 13_4_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.5 Safari/605.1.15",
        "Mozilla/5.0 (X11; Linux x86_64) Gecko/20100101 Firefox/117.0",
    ];

    let mut result = String::new();

    match duckduckgo_search(query, &user_agents).await {
        Ok(urls) => {
            println!("Top results for '{}':", query);
            for url in urls.iter().take(3) {
                println!("- {}", url);
                match scrape_url(url).await {
                    Ok(text) => result.push_str(&text),
                    Err(e) => eprintln!("❌ Failed to scrape {}: {}", url, e),
                }
            }
        }
        Err(e) => {
            eprintln!("Search failed: {}", e);
        }
    }

    result
}

async fn scrape_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let body = response.text().await?;

    let document = Html::parse_document(&body);
    let tags = vec!["p"];

    let mut content = String::new();

    for tag in tags {
        let selector = Selector::parse(tag)?;
        for element in document.select(&selector) {
            let text = element.text().collect::<Vec<_>>().join(" ");
            if text.trim().is_empty() {
                continue;
            } else if text.trim().split_whitespace().count() < 5 {
                continue;
            } else if !(text.trim().ends_with('.') || text.trim().ends_with('!') ||
                text.trim().ends_with('?')) {
                continue;
            }
            content.push_str(&format!("{}\n", text.trim()));
        }
    }

    Ok(content)
}

