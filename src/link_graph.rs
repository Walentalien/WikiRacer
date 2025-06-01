use blake3;
use std::collections::HashMap;

/// Deterministic hash for a URL
fn hash_url(url: &str) -> String {
    blake3::hash(url.as_bytes()).to_hex().to_string()
}

/// Link struct as before
#[derive(Debug)]
struct Link {
    url: String,
    children: Vec<String>, // Store children by hashed ID
    parents: Vec<String>,
}

struct LinkGraph {
    links: HashMap<String, Link>, // key: hash of URL
}

impl LinkGraph {
    fn new() -> Self {
        Self { links: HashMap::new() }
    }

    fn add_link(&mut self, url: &str) {
        let id = hash_url(url);
        self.links.entry(id.clone()).or_insert(Link {
            url: url.to_string(),
            children: vec![],
            parents: vec![],
        });
    }

    fn add_parent(&mut self, child_url: &str, parent_url: &str) {
        let child_id = hash_url(child_url);
        let parent_id = hash_url(parent_url);

        self.links.entry(child_id.clone()).or_insert_with(|| Link {
            url: child_url.to_string(),
            children: vec![],
            parents: vec![],
        });

        self.links.entry(parent_id.clone()).or_insert_with(|| Link {
            url: parent_url.to_string(),
            children: vec![],
            parents: vec![],
        });

        // Update relationships
        self.links.get_mut(&child_id).unwrap().parents.push(parent_id.clone());
        self.links.get_mut(&parent_id).unwrap().children.push(child_id.clone());
    }
}
