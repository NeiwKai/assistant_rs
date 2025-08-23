mod duckduckgo;
use crate::duckduckgo::duckduckgo_search;

#[tokio::main]
async fn websearch(query: &str) {
    //let query = "gentoo linux";

    match duckduckgo_search(query).await {
        Ok(results) => {
            println!("Top results for '{}':", query);
            for url in results.iter().take(5) {
                println!("- {}", url);
            }
        }
        Err(e) => {
            eprintln!("Search failed: {}", e);
        }
    }
}
