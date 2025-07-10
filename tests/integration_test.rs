use std::sync::Arc;
use url::Url;
use WikiRacer::{config, pathfinder};
use WikiRacer::crawler;
use WikiRacer::crawler::CrawlerConfig;

#[tokio::test]
async fn test_wikipedia_path_finding() -> Result<(), Box<dyn std::error::Error>> {
    let start_url = Url::parse("https://en.wikipedia.org/wiki/Matter")?;
    let target_url = Url::parse("https://en.wikipedia.org/wiki/Chemistry")?;

    let config = Arc::new(
        CrawlerConfig::new(start_url.clone())
            .with_max_urls(300)
            .with_max_depth(2)
            .with_thread_count(4)
            .with_request_delay(100),
    );

    let state = Arc::new(crawler::CrawlerState::new(start_url.clone()));

    crawler::crawl(state.clone(), config).await?;

    let graph = crawler::build_graph_from_state(&state);
    let graph = crawler::build_graph_from_state(&state).await;

    // Find shortest path
    let path = pathfinder::find_shortest_path_bfs(&start_url, &target_url, &graph);

    match path {
        Some(path) => {
            assert!(path.len() <= 3, "Path should be at most 2 steps (direct link expected)");
            assert_eq!(path[0], start_url);
            assert_eq!(path[path.len() - 1], target_url);
        }
        None => panic!("Expected to find a path between Matter and Chemistry"),
    }

    Ok(())
}