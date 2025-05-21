use ada_rust::{parse, UrlAggregator, SchemeType, HostType, UrlComponents, ParseError};

#[test]
fn initial_parse_test() {
    // This test expects the parser to make some progress.
    // With Scheme, Authority, Host, Port states implemented, it should parse simple URLs.
    let result = parse("https://www.google.com", None);
    match result {
        Ok(_url_aggregator) => {
            assert!(true); 
        }
        Err(e) => {
            assert!(false, "Parser returned an error for a valid URL: {:?}", e);
        }
    }
}

#[test]
fn test_simple_http_url() {
    let result = parse("http://example.com/", None);
    assert!(result.is_ok(), "Parsing failed: {:?}", result.err());
    let url = result.unwrap();
    assert_eq!(url.scheme_type(), SchemeType::Http);
    assert!(url.is_special());
    
    assert_eq!(url.buffer, "http://example.com/"); // Assuming Path state appends the final slash

    assert_eq!(url.components.protocol_end, "http:".len() as u32);
    assert_eq!(url.components.host_start, "http://".len() as u32); 
    assert_eq!(url.components.host_end, "http://example.com".len() as u32);
    assert_eq!(url.host_type(), HostType::Default); // Assuming not IP
    assert_eq!(url.components.port_value, None);
    assert_eq!(url.components.pathname_start, "http://example.com".len() as u32); // Path starts after host
}

#[test]
fn test_http_url_with_port() {
    let result = parse("https://example.com:8080/path", None);
    assert!(result.is_ok(), "Parsing failed: {:?}", result.err());
    let url = result.unwrap();
    assert_eq!(url.scheme_type(), SchemeType::Https);
    assert!(url.is_special());

    // Buffer check depends on how much of path parsing is done.
    // "https://example.com:8080/path"
    assert_eq!(url.buffer, "https://example.com:8080/path");

    assert_eq!(url.components.protocol_end, "https:".len() as u32);
    assert_eq!(url.components.host_start, "https://".len() as u32);
    assert_eq!(url.components.host_end, "https://example.com:8080".len() as u32); // host_end includes port string
    assert_eq!(url.components.port_value, Some(8080));
    assert_eq!(url.components.pathname_start, "https://example.com:8080".len() as u32);
}

#[test]
fn test_url_with_userinfo() {
    let result = parse("ftp://user:password@example.com/", None);
    assert!(result.is_ok(), "Parsing failed: {:?}", result.err());
    let url = result.unwrap();
    assert_eq!(url.scheme_type(), SchemeType::Ftp);
    assert!(url.is_special());

    assert_eq!(url.buffer, "ftp://user:password@example.com/");

    assert_eq!(url.components.protocol_end, "ftp:".len() as u32);
    // username_end in UrlAggregator is currently set to end of password if present.
    assert_eq!(url.components.username_end, "ftp://user:password".len() as u32); 
    assert_eq!(url.components.host_start, "ftp://user:password@".len() as u32);
    assert_eq!(url.components.host_end, "ftp://user:password@example.com".len() as u32);
    assert_eq!(url.components.pathname_start, "ftp://user:password@example.com".len() as u32);
}

#[test]
fn test_simple_file_url() {
    let result = parse("file:///path/to/file", None);
    assert!(result.is_ok(), "Parsing failed: {:?}", result.err());
    let url = result.unwrap();
    assert_eq!(url.scheme_type(), SchemeType::File);
    assert!(url.is_special());

    assert_eq!(url.buffer, "file:///path/to/file");

    assert_eq!(url.components.protocol_end, "file:".len() as u32);
    assert_eq!(url.components.host_start, "file://".len() as u32);
    // For "file:///path", host is empty. set_host_from_slice("") is called.
    assert_eq!(url.components.host_end, "file://".len() as u32); // Empty host means host_end == host_start
    assert_eq!(url.host_type(), HostType::Default); // Empty host
    assert_eq!(url.components.pathname_start, "file://".len() as u32); // Path starts after the authority marker for empty host
}

#[test]
fn test_url_with_username_only() {
    let result = parse("http://user@example.com/", None);
    assert!(result.is_ok(), "Parsing failed: {:?}", result.err());
    let url = result.unwrap();
    assert_eq!(url.scheme_type(), SchemeType::Http);

    assert_eq!(url.buffer, "http://user@example.com/");

    assert_eq!(url.components.protocol_end, "http:".len() as u32);
    // username_end is set to end of username if no password
    assert_eq!(url.components.username_end, "http://user".len() as u32); 
    assert_eq!(url.components.host_start, "http://user@".len() as u32); 
    assert_eq!(url.components.host_end, "http://user@example.com".len() as u32);
    assert_eq!(url.components.pathname_start, "http://user@example.com".len() as u32);
}
