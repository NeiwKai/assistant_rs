mod websearch;
use crate::websearch::websearch;

mod duckduckgo;

//#[tokio::main]
async fn get_result() {
    let result = websearch("gentoo linux").await;
    println!("\n=== Scraped Content ===\n{}", result);
}

