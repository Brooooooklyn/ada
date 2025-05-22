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

impl SchemeType {
    pub fn is_special(&self) -> bool {
        matches!(self, Self::Http | Self::Https | Self::File | Self::Ftp | Self::Ws | Self::Wss)
    }

    pub fn default_port(&self) -> Option<u16> {
        match self {
            SchemeType::Http | SchemeType::Ws => Some(80),
            SchemeType::Https | SchemeType::Wss => Some(443),
            SchemeType::Ftp => Some(21),
            _ => None,
        }
    }
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
    pub username_end: u32,   // End of username in buffer
    pub host_start: u32,     // Start of host in buffer (after username/password)
    pub host_end: u32,       // End of host in buffer (before port, path)
    // pub port: Option<u16>, // Removed, replaced by port_value.
    pub port_value: Option<u16>, // The actual parsed port number
    pub pathname_start: u32,   // Start of path in buffer
    pub search_start: Option<u32>, // Start of search string (query) in buffer, after '?'
    pub hash_start: Option<u32>,   // Start of fragment in buffer, after '#'
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
    // UrlBase::port() should now reflect the port_value from UrlComponents
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
