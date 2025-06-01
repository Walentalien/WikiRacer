
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::Arc;
    use url::Url;

    #[tokio::test]
    #[ignore] // Run with --ignored flag when testing against real Wikipedia
    async fn test_wikipedia_path_finding() -> Result<(), Box<dyn std::error::Error>> {
        let start_url = Url::parse("https://en.wikipedia.org/wiki/Matter")?;
        let target_url = Url::parse("https://en.wikipedia.org/wiki/Grand_Unified_Theory")?;

        let config = Arc::new(
            crate::crawler::CrawlerConfig::new(start_url.clone())
                .with_max_urls(1000)
                .with_max_depth(2)
                .with_thread_count(4)
                .with_request_delay(100)
        );

        let state = Arc::new(crate::crawler::CrawlerState::new(start_url.clone()));

        // Crawl the pages
        crate::crawler::crawl(state.clone(), config).await?;

        // Build graph from crawled data
        let graph = crate::crawler::build_graph_from_state(&state);

        println!("Graph has {} nodes", graph.len());

        // Check if target is reachable
        if let Some(children) = graph.get(&start_url) {
            println!("Start page has {} outgoing links", children.len());
            if children.contains(&target_url) {
                println!("Direct link found from Matter to Grand Unified Theory!");
            }
        }

        // Find shortest path
        let path = crate::pathfinder::find_shortest_path(&start_url, &target_url, &graph);

        match path {
            Some(path) => {
                println!("Path found with {} steps:", path.len() - 1);
                for (i, url) in path.iter().enumerate() {
                    println!("  {}: {}", i, url);
                }
                assert!(path.len() <= 3, "Path should be at most 2 steps (direct link expected)");
                assert_eq!(path[0], start_url);
                assert_eq!(path[path.len() - 1], target_url);
            }
            None => {
                // Print some debug info
                println!("No path found. Debug info:");
                println!("Start URL in graph: {}", graph.contains_key(&start_url));
                println!("Target URL in graph: {}", graph.contains_key(&target_url));

                if let Some(children) = graph.get(&start_url) {
                    println!("Links from start page (first 10):");
                    for (i, child) in children.iter().take(10).enumerate() {
                        println!("  {}: {}", i, child);
                    }
                }

                panic!("Expected to find a path between Matter and Grand Unified Theory");
            }
        }

        Ok(())
    }


}
