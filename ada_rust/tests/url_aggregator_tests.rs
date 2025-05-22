use ada_rust::{parse, UrlAggregator, SchemeType, HostType, UrlComponents, ParseError};

#[test]
fn test_clear_pathname_full_url() {
    let mut url = parse("http://example.com/path/to/resource?query=1#fragment", None).unwrap();
    url.clear_pathname();
    assert_eq!(url.href(), "http://example.com/?query=1#fragment"); // Path becomes "/"
    assert_eq!(url.pathname(), "/");
    assert_eq!(url.search(), "?query=1");
    assert_eq!(url.hash(), "#fragment");
    assert!(!url.has_opaque_path());
}

#[test]
fn test_clear_pathname_no_query_fragment() {
    let mut url = parse("http://example.com/path/to/resource", None).unwrap();
    url.clear_pathname();
    assert_eq!(url.href(), "http://example.com/"); // Path becomes "/"
    assert_eq!(url.pathname(), "/");
    assert_eq!(url.search(), ""); 
    assert_eq!(url.hash(), "");   
}

#[test]
fn test_clear_pathname_only_query() {
    let mut url = parse("http://example.com/path?query=1", None).unwrap();
    url.clear_pathname();
    assert_eq!(url.href(), "http://example.com/?query=1");
    assert_eq!(url.pathname(), "/");
    assert_eq!(url.search(), "?query=1");
}

#[test]
fn test_clear_pathname_only_fragment() {
    let mut url = parse("http://example.com/path#fragment", None).unwrap();
    url.clear_pathname();
    assert_eq!(url.href(), "http://example.com/#fragment");
    assert_eq!(url.pathname(), "/");
    assert_eq!(url.hash(), "#fragment");
}

#[test]
fn test_clear_pathname_no_authority() {
    // For "mailto:user@example.com/pathpart", parser currently makes path part of opaque path.
    // clear_pathname on such a URL will clear after "mailto:".
    // The parser sets has_opaque_path = true for "mailto:user@example.com/pathpart"
    // clear_pathname sets has_opaque_path = false.
    // The path becomes empty (not even "/").
    let mut url = parse("mailto:user@example.com/pathpart", None).unwrap();
    url.clear_pathname(); 
    assert_eq!(url.href(), "mailto:user@example.com"); 
    assert_eq!(url.pathname(), ""); // Path is empty for opaque schemes after clear
    assert!(!url.has_opaque_path()); 
}

#[test]
fn test_clear_pathname_empty_path_initially() {
    // Parser behavior for "http://example.com?query":
    // Scheme: "http", Authority: "example.com", Path: (empty, becomes "/"), Query: "query"
    let mut url = parse("http://example.com?query", None).unwrap();
    // pathname is already "/" due to parser logic for special URLs with authority.
    // clear_pathname should effectively result in the same state if path was already "/".
    url.clear_pathname(); 
    assert_eq!(url.href(), "http://example.com/?query");
    assert_eq!(url.pathname(), "/"); 
}

#[test]
fn test_clear_search_with_fragment() {
    let mut url = parse("http://example.com/path?query=1#fragment", None).unwrap();
    url.clear_search();
    assert_eq!(url.href(), "http://example.com/path#fragment");
    assert_eq!(url.search(), "");
    assert_eq!(url.hash(), "#fragment");
}

#[test]
fn test_clear_search_no_fragment() {
    let mut url = parse("http://example.com/path?query=1", None).unwrap();
    url.clear_search();
    assert_eq!(url.href(), "http://example.com/path");
    assert_eq!(url.search(), "");
}

#[test]
fn test_clear_search_no_search_initially() {
    let mut url = parse("http://example.com/path#fragment", None).unwrap();
    url.clear_search(); // Should do nothing
    assert_eq!(url.href(), "http://example.com/path#fragment");
}

#[test]
fn test_clear_hash() {
    let mut url = parse("http://example.com/path?query=1#fragment", None).unwrap();
    url.clear_hash();
    assert_eq!(url.href(), "http://example.com/path?query=1");
    assert_eq!(url.hash(), "");
}

#[test]
fn test_clear_hash_no_hash_initially() {
    let mut url = parse("http://example.com/path?query=1", None).unwrap();
    url.clear_hash(); // Should do nothing
    assert_eq!(url.href(), "http://example.com/path?query=1");
}
