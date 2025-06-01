
use std::collections::{HashMap, VecDeque, HashSet};
use url::Url;
use anyhow::Result;

/// Find shortest path between two URLs using BFS
pub fn find_shortest_path(
    start: &Url,
    target: &Url,
    graph: &HashMap<Url, Vec<Url>>
) -> Option<Vec<Url>> {
    if start == target {
        return Some(vec![start.clone()]);
    }

    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut parent: HashMap<Url, Url> = HashMap::new();

    queue.push_back(start.clone());
    visited.insert(start.clone());

    while let Some(current) = queue.pop_front() {
        if let Some(neighbors) = graph.get(&current) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    visited.insert(neighbor.clone());
                    parent.insert(neighbor.clone(), current.clone());
                    queue.push_back(neighbor.clone());

                    if neighbor == target {
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
