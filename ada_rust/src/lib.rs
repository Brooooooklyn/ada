// Define public enum SchemeType
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemeType {
    Http,
    Https,
    File,
    Ftp,
    Ws,
    Wss,
    NotSpecial,
}

// Define public enum HostType
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostType {
    Default,
    Ipv4,
    Ipv6,
}

// Define public struct UrlComponents
#[derive(Debug, Default, PartialEq, Eq)]
pub struct UrlComponents {
    pub protocol_end: u32,
    pub username_end: u32,
    pub host_start: u32,
    pub host_end: u32,
    pub port: Option<u16>,
    pub pathname_start: u32,
    pub search_start: Option<u32>,
    pub hash_start: Option<u32>,
}

// Define public trait UrlBase
pub trait UrlBase {
    fn is_valid(&self) -> bool;
    fn has_opaque_path(&self) -> bool;
    fn scheme_type(&self) -> SchemeType;
    fn host_type(&self) -> HostType;
    fn is_special(&self) -> bool {
        match self.scheme_type() {
            SchemeType::Http | SchemeType::Https | SchemeType::File | SchemeType::Ftp | SchemeType::Ws | SchemeType::Wss => true,
            SchemeType::NotSpecial => false,
        }
    }
    fn origin(&self) -> String;
    fn protocol_end(&self) -> u32;
    fn username_end(&self) -> u32;
    fn host_start(&self) -> u32;
    fn host_end(&self) -> u32;
    fn port(&self) -> Option<u16>;
    fn pathname_start(&self) -> u32;
    fn search_start(&self) -> Option<u32>;
    fn hash_start(&self) -> Option<u32>;
    fn to_legacy_string(&self) -> String;
}

pub mod url_aggregator;
pub use url_aggregator::UrlAggregator;

pub mod parser;
pub use parser::parse;
pub use parser::ParseError;

pub mod url;
pub use url::AdaUrl;

// Original add function and tests for now
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
