# WikiRacer
Find Shortest Path Between Two Links on Wikipedia


_| Macro      | Description                                    | Typical Use Case                    |
| ---------- | ---------------------------------------------- | ----------------------------------- |
| `trace!()` | Very detailed info (e.g., every function call) | Debugging low-level flows           |
| `debug!()` | General debugging info                         | Values of variables, flow checks    |
| `info!()`  | High-level state/info                          | Starting app, finished task, status |
| `warn!()`  | Something unexpected, but non-fatal            | Slow network, fallback in use       |
| `error!()` | Fatal error occurred                           | Failed connection, file not found_   |


TODO:
1. Crawl function
2. Data Structures to store scraped data
Argument Parsing 
Pretty Print Args
GUI for this crawler
start using anychow

Creating graph visualization: https://doc.arcgis.com/en/insights/latest/create/link-chart.htm


Performance Optimizations:
- Implement parallel crawling more efficiently using a work-stealing thread pool
- Add caching for visited pages to avoid re-crawling
- Use a more efficient data structure for storing the graph (consider using a specialized graph library like petgraph)
- Implement request rate limiting with a token bucket algorithm instead of fixed delays


Code Structure Improvements:
Split the large crawler.rs (671 lines) into smaller, more focused modules
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

# Contribution GuideLine
Open for Contribution
Looking for help with:
- GUI Application
- Path Finding/Link Processing Algorithm Improvement
- Visualization of graph/ of graph creation (process)
- Adding relevance evaluation to make the process quicker
- Caching/Caching logic 
- 