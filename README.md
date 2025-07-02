# WikiRacer
Find Shortest Path Between Two Links on Wikipedia
## RUNNING
```bash
cargo build
cargo run -- --start-url <URL> --target-url <URL>
```

### CLI Options
The available flags are:

* `-s, --start-url <URL>` – starting Wikipedia page
* `--target-url <URL>` – target Wikipedia page
* `--max-urls <NUM>` – maximum pages to crawl (default: 1000)
* `--max-depth <NUM>` – maximum crawl depth (default: 3)
* `--thread-count <NUM>` – number of crawler threads (default: 4)
* `-r, --request-delay <MS>` – delay between requests in milliseconds (default: 100)
* `-o, --output-file <PATH>` – write the graph to a file
* `-v, --verbose` – enable verbose logging

TODO:
1. FIX CONCURENCY ISSUES
2. GUI for this crawler
3. Creating graph visualization: https://doc.arcgis.com/en/insights/latest/create/link-chart.htm
4. Fix RwLock
5. 

Performance Optimizations:
- Implement parallel crawling more efficiently using a work-stealing thread pool
- Add caching for visited pages to avoid re-crawling
- Use a more efficient data structure for storing the graph (consider using a specialized graph library like petgraph)
- Implement request rate limiting with a token bucket algorithm instead of fixed delays


Code Structure Improvements:
Split the large crawler.rs into smaller, more focused modules
Create a dedicated configuration module for better configuration management
Implement proper error handling with custom error types
Add more comprehensive logging and metrics collection
Feature Enhancements:
Add progress reporting during crawling
Implement a proper CLI interface using clap instead of basic argument parsing
Add support for different Wikipedia languages
Implement a proper GUI using a framework like egui or iced
Add visualization of the crawling process and path finding


Testing and Reliability:
Add more unit tests for core functionality
Implement integration tests for the full crawling process
Add property-based testing for the path finding algorithm
Implement proper error recovery mechanisms
Documentation and User Experience:
Add comprehensive API documentation
Create a proper user guide
Add examples for common use cases
Implement better progress reporting and user feedback
Here's a specific implementation plan to achieve these improvements:

# Contribution Guideline
**Open for Contribution**
Looking for help with:
- GUI Application
- Path Finding/Link Processing Algorithm Improvement
- **Visualization** of graph creation process

## License
This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.