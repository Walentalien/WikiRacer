
use std::collections::{HashMap, VecDeque, HashSet};
use url::Url;

/// Find shortest path between two URLs using BFS
pub fn find_shortest_path_bfs(
    start: &Url,
    target: &Url,
    graph: &HashMap<Url, HashSet<Url>>,
) -> Option<Vec<Url>> {
    use log2::debug;

    debug!("Searching for path from {} to {}", start, target);
    debug!("Graph contains {} nodes", graph.len());

    if start == target {
        return Some(vec![start.clone()]);
    }

    if !graph.contains_key(start) {
        debug!("Start URL not found in graph");
        return None;
    }

    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut parent: HashMap<Url, Url> = HashMap::new();

    queue.push_back(start.clone());
    visited.insert(start.clone());

    let mut depth = 0;
    let mut nodes_at_current_depth = 1;
    let mut nodes_processed_at_current_depth = 0;

    while let Some(current) = queue.pop_front() {
        nodes_processed_at_current_depth += 1;

        if nodes_processed_at_current_depth > nodes_at_current_depth {
            depth += 1;
            nodes_at_current_depth = queue.len();
            nodes_processed_at_current_depth = 1;
            debug!("Searching at depth {}, {} nodes in queue", depth, queue.len());
        }

        if let Some(neighbors) = graph.get(&current) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    visited.insert(neighbor.clone());
                    parent.insert(neighbor.clone(), current.clone());
                    queue.push_back(neighbor.clone());

                    if neighbor == target {
                        debug!("Found target at depth {}", depth + 1);
                        // Reconstruct path
                        let mut path = Vec::new();
                        let mut node = target.clone();

                        while let Some(prev) = parent.get(&node) {
                            path.push(node.clone());
                            node = prev.clone();
                        }
                        path.push(start.clone());
                        path.reverse();

                        return Some(path);
                    }
                }
            }
        }
    }

    debug!("No path found after searching {} nodes", visited.len());
    None // No path found
}

/// Print the path in a readable format
pub fn print_path(path: &[Url]) {
    println!("Shortest path ({} steps):", path.len() - 1);
    for (i, url) in path.iter().enumerate() {
        if i == 0 {
            println!("  START: {}", url);
        } else if i == path.len() - 1 {
            println!("  END:   {}", url);
        } else {
            println!("  {}:     {}", i, url);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;
    use std::collections::HashMap;

    fn url(path: &str) -> Url {
        Url::parse(&format!("https://example.com{}", path)).unwrap()
    }

    fn setup_graph() -> HashMap<Url, HashSet<Url>> {
        let a = url("/a");
        let b = url("/b");
        let c = url("/c");
        let d = url("/d");
        let e = url("/e");

        // HashMap::from([
        //     (a.clone(), vec![b.clone(), c.clone()]),
        //     (b.clone(), vec![d.clone()]),
        //     (c.clone(), vec![d.clone()]),
        //     (d.clone(), vec![e.clone()]),
        //     (e.clone(), vec![]),
        // ])
        use std::collections::{HashMap, HashSet};

        HashMap::from([
            (a.clone(), vec![b.clone(), c.clone()].into_iter().collect()),
            (b.clone(), vec![d.clone()].into_iter().collect()),
            (c.clone(), vec![d.clone()].into_iter().collect()),
            (d.clone(), vec![e.clone()].into_iter().collect()),
            (e.clone(), HashSet::new()),
        ])

    }

    #[test]
    fn test_path_exists() {
        let graph = setup_graph();
        let start = url("/a");
        let target = url("/e");

        let path = find_shortest_path_bfs(&start, &target, &graph).unwrap();
        assert_eq!(path.first().unwrap(), &start);
        assert_eq!(path.last().unwrap(), &target);
        assert!(path.len() >= 3); // at least a → ... → e
    }

    #[test]
    fn test_start_equals_target() {
        let graph = setup_graph();
        let start = url("/a");

        let path = find_shortest_path_bfs(&start, &start, &graph).unwrap();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], start);
    }

    #[test]
    fn test_no_path() {
        let mut graph = setup_graph();
        // disconnect /a from the rest
        graph.insert(url("/a"), HashSet::new());

        let result = find_shortest_path_bfs(&url("/a"), &url("/e"), &graph);
        assert!(result.is_none());
    }

    #[test]
    fn test_missing_start() {
        let graph = setup_graph();
        let unknown = url("/missing");

        let result = find_shortest_path_bfs(&unknown, &url("/e"), &graph);
        assert!(result.is_none());
    }

    #[test]
    fn test_cycle() {
        let a = url("/a");
        let b = url("/b");

        let graph = HashMap::from([
            (a.clone(), (vec![b.clone()]).into_iter().collect()),
            (b.clone(), (vec![a.clone()]).into_iter().collect()), // cycle
        ]);

        let path = find_shortest_path_bfs(&a, &b, &graph).unwrap();
        assert_eq!(path, vec![a, b]);
    }
}
