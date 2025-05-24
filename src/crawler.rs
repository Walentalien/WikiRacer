use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
// Crawler Logic
use url::Url;

const LINK_REQUEST_TIMEOUT_SEC: u64 = 2;

/// Configuration of current state of the crawler
pub struct CrawlerState {
    pub links_crawled_count: usize,
    pub link_to_crawl_queue: RwLock<VecDeque<Url>>,
    pub links_graph: RwLock<Vec<Link>>
}

pub struct Link {
    pub id: LinkID,
    pub url: String,
    pub children: Vec<Link>,
    pub parents: Vec<Link>
}

pub type LinkID = usize;
// Arc for simultaneos access by multiple threads
pub type CrawlerStateRef = Arc<CrawlerState>;
pub struct CrawlerConfig{
    pub starting_url: Url,
}
/// If `path` is full_url -> returns it
/// if relative constructs full url  by merging with `root_url`
/// If there is trailing slash it removes it
fn construct_url(path: &str, root_url: Url) -> Result<Url, url::ParseError> {
    let mut url = if let Ok(parsed_url) = Url::parse(path) {
        if parsed_url.host().is_some() {
            parsed_url
        } else {
            root_url.join(path)?
        }
    } else {
        root_url.join(path)?
    };

    // Normalize: trim trailing slashes from path and fragment
    let trimmed_path = url.path().trim_end_matches('/').to_string();
    url.set_path(&trimmed_path);

    if let Some(frag) = url.fragment().map(|s| s.to_string()) {
        url.set_fragment(Some(frag.trim_end_matches('/')));
    }

    Ok(url)
}




#[cfg(test)]
mod tests {
    use url::Url;
    use crate::crawler::construct_url;


    // tests for construct_url start here
    #[test]
    fn test_get_url() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://google.com")?;
        let another_full_url = Url::parse("https://anotherlink.com");
        let result_url = construct_url(another_full_url.clone().unwrap().as_str(), root_url)?;
        assert_eq!(result_url, another_full_url.unwrap());
        Ok(())
    }

    #[test]
    fn test_get_url_with_relative_path() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://groq.xyz")?;
        // for the path arg we have relative path
        //let relative_path = Url::parse("/some/relative/path")?;
        let relative_path =  "/some/relative/path";
        let result_url = construct_url(relative_path.clone(), root_url )?;
        let expected_url = Url::parse("https://groq.xyz/some/relative/path")?;
        assert_eq!(result_url, expected_url);
        Ok(())
    }

    #[test]
    fn test_malformed_url() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://valid.root.url")?;
        let malformed_url = "this_is_a_malformed_url";
        let result_url = construct_url(malformed_url, root_url);
        assert_eq!("https://valid.root.url/this_is_a_malformed_url", result_url.unwrap().to_string());
        Ok(())
    }

    #[test]
    fn test_empty_path() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://groq.xyz")?;
        let result_url = construct_url("", root_url)?;
        let expected_url = Url::parse("https://groq.xyz/")?;
        assert_eq!(result_url, expected_url);
        Ok(())
    }
    #[test]
    fn test_url_with_query() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://groq.xyz")?;
        let relative_path = "/some/relative/path?query=1";
        let result_url = construct_url(relative_path, root_url)?;
        let expected_url = Url::parse("https://groq.xyz/some/relative/path?query=1")?;
        assert_eq!(result_url, expected_url);
        Ok(())
    }
    #[test]
    fn test_hash_fragment() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://groq.xyz")?;
        let relative_path = "/some/relative/path#section/";
        let result_url = construct_url(relative_path, root_url)?;
        let expected_url = Url::parse("https://groq.xyz/some/relative/path#section")?;
        assert_eq!(result_url, expected_url);
        Ok(())
    }

    fn test_get_url_with_link_with_trailing_slash() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://groq.xyz")?;

        let relative_path_wo_trailing_slash = "/some/relative/path";
        let relative_path_w_trailing_slash = "/some/relative/path/";

        let result_wo_trailing_slash = construct_url(relative_path_wo_trailing_slash, root_url.clone())?;
        let result_w_trailing_slash = construct_url(relative_path_w_trailing_slash, root_url)?;

        assert_eq!(result_wo_trailing_slash, result_w_trailing_slash);
        Ok(())
    }
    // tests for construct_url end here

}