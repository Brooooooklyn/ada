use super::{UrlBase, SchemeType, HostType};
use crate::url_aggregator::SetError;

// Define public struct AdaUrl
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaUrl {
    // Public fields for URL components
    pub protocol: String,
    pub username: String,
    pub password: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub pathname: String,
    pub search: Option<String>,
    pub hash: Option<String>,

    // Private fields corresponding to UrlBase properties
    is_valid: bool,
    has_opaque_path: bool,
    scheme_type: SchemeType,
    host_type: HostType,
}

impl AdaUrl {
    // Public constructor
    pub fn new(
        protocol: String,
        username: String,
        password: String,
        host: Option<String>,
        port: Option<u16>,
        pathname: String,
        search: Option<String>,
        hash: Option<String>,
        is_valid: bool,
        has_opaque_path: bool,
        scheme_type: SchemeType,
        host_type: HostType,
    ) -> Self {
        Self {
            protocol,
            username,
            password,
            host,
            port,
            pathname,
            search,
            hash,
            is_valid,
            has_opaque_path,
            scheme_type,
            host_type,
        }
    }

    // Getters
    pub fn href(&self) -> String {
        self.to_legacy_string() // Or "todo_href()"
    }

    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    pub fn hostname(&self) -> Option<&str> {
        // Placeholder, same as host for now
        self.host.as_deref()
    }

    // Note: This overrides the port() method from UrlBase trait that returns Option<u16>
    // If a &str version is needed, it should be named differently or the trait method adjusted.
    // For now, let's assume this is the intended getter for the port as a string.
    // However, the task asks for Option<u16>, which is already provided by UrlBase impl.
    // Re-checking the task: "pub fn port(&self) -> Option<u16>: Should return self.port."
    // This is ALREADY implemented by the UrlBase trait's port method.
    // So, no new port() getter here unless it's meant to be different (e.g. returns &str).
    // The UrlBase trait already provides: fn port(&self) -> Option<u16> { self.port }
    // I will skip adding a duplicate pub fn port(&self) -> Option<u16> here.

    pub fn pathname(&self) -> &str {
        &self.pathname
    }

    pub fn search(&self) -> Option<&str> {
        self.search.as_deref()
    }

    pub fn hash(&self) -> Option<&str> {
        self.hash.as_deref()
    }

    // Setters
    pub fn set_href(&mut self, _input: &str) -> Result<(), SetError> {
        Err(SetError::InvalidInput)
    }

    pub fn set_protocol(&mut self, input: &str) -> Result<(), SetError> {
        self.protocol = input.to_string();
        // Potentially update self.scheme_type based on the new protocol
        Ok(())
    }

    pub fn set_username(&mut self, input: &str) -> Result<(), SetError> {
        self.username = input.to_string();
        Ok(())
    }

    pub fn set_password(&mut self, input: &str) -> Result<(), SetError> {
        self.password = input.to_string();
        Ok(())
    }

    pub fn set_host(&mut self, input: Option<&str>) -> Result<(), SetError> {
        self.host = input.map(|s| s.to_string());
        // Potentially update self.host_type
        Ok(())
    }

    pub fn set_hostname(&mut self, input: Option<&str>) -> Result<(), SetError> {
        // Placeholder, similar to set_host
        self.host = input.map(|s| s.to_string());
        // Potentially update self.host_type
        Ok(())
    }

    pub fn set_port(&mut self, input: Option<&str>) -> Result<(), SetError> {
        if let Some(port_str) = input {
            // Attempt to parse, return error if fails or if non-empty and invalid
            // For now, placeholder logic: if input is Some, return error.
             Err(SetError::InvalidInput)
        } else {
            self.port = None;
            Ok(())
        }
    }

    pub fn set_pathname(&mut self, input: &str) -> Result<(), SetError> {
        self.pathname = input.to_string();
        Ok(())
    }

    pub fn set_search(&mut self, input: Option<&str>) -> Result<(), SetError> {
        self.search = input.map(|s| s.to_string());
        Ok(())
    }

    pub fn set_hash(&mut self, input: Option<&str>) -> Result<(), SetError> {
        self.hash = input.map(|s| s.to_string());
        Ok(())
    }

    // Revised get_origin for clarity and correctness:
    pub fn get_origin(&self) -> String {
        if !self.is_valid {
            return "null".to_string();
        }

        match self.scheme_type {
            super::SchemeType::File => {
                "null".to_string()
            }
            super::SchemeType::Http |
            super::SchemeType::Https |
            super::SchemeType::Ftp |
            super::SchemeType::Ws |
            super::SchemeType::Wss => {
                let host_val = match &self.host {
                    Some(h) if !h.is_empty() => h.as_str(),
                    _ => return "null".to_string(), // Opaque origin if no host
                };

                // TODO: Host should be ASCII lowercase if it's a domain.
                // IDNA processing would also happen here in a full implementation.
                // For now, use as is.

                let mut origin_str = format!("{}://{}", self.protocol, host_val);

                if let Some(p) = self.port {
                    if Some(p) != self.scheme_type.default_port() { // default_port() is on SchemeType
                        origin_str.push_str(&format!(":{}", p));
                    }
                }
                origin_str
            }
            _ => {
                // For other schemes, including blob, or if is_valid was false initially.
                "null".to_string()
            }
        }
    }
}

impl UrlBase for AdaUrl {
    fn is_valid(&self) -> bool {
        self.is_valid
    }

    fn has_opaque_path(&self) -> bool {
        self.has_opaque_path
    }

    fn scheme_type(&self) -> SchemeType {
        self.scheme_type
    }

    fn host_type(&self) -> HostType {
        self.host_type
    }

    fn is_special(&self) -> bool {
        match self.scheme_type {
            SchemeType::Http | SchemeType::Https | SchemeType::File | SchemeType::Ftp | SchemeType::Ws | SchemeType::Wss => true,
            SchemeType::NotSpecial => false,
        }
    }

    fn origin(&self) -> String {
        // Placeholder
        "todo_origin()".to_string()
    }

    // These methods relate to the UrlComponents struct which AdaUrl does not directly use in the same way
    // as UrlAggregator. For AdaUrl, these would typically be derived from its own fields.
    // For now, returning default/dummy values as their direct meaning is different for AdaUrl.
    fn protocol_end(&self) -> u32 {
        // This would be calculated based on self.protocol.len() if it included ':',
        // but for now, let's assume it's just the scheme length.
        self.protocol.len() as u32 
    }

    fn username_end(&self) -> u32 {
        // Placeholder, this is not directly stored as an index.
        0 
    }

    fn host_start(&self) -> u32 {
        // Placeholder
        0
    }

    fn host_end(&self) -> u32 {
        // Placeholder
        0
    }

    // port() from UrlBase returns Option<u16>, which matches AdaUrl's port field.
    fn port(&self) -> Option<u16> { // Corrected u116 to u16
        self.port
    }

    fn pathname_start(&self) -> u32 {
        // Placeholder
        0
    }

    fn search_start(&self) -> Option<u32> {
        // Placeholder
        None
    }

    fn hash_start(&self) -> Option<u32> {
        // Placeholder
        None
    }
    
    fn to_legacy_string(&self) -> String {
        // Placeholder for eventual JSON-like string or full URL string reconstruction.
        // For now, a simple concatenation for debugging.
        let mut s = self.protocol.clone();
        s.push_str("://");
        if !self.username.is_empty() || !self.password.is_empty() {
            s.push_str(&self.username);
            if !self.password.is_empty() {
                s.push(':');
                s.push_str(&self.password);
            }
            s.push('@');
        }
        if let Some(h) = &self.host {
            s.push_str(h);
        }
        if let Some(p) = self.port {
            s.push(':');
            s.push_str(&p.to_string());
        }
        s.push_str(&self.pathname);
        if let Some(search_query) = &self.search {
            s.push('?');
            s.push_str(search_query);
        }
        if let Some(h) = &self.hash {
            s.push('#');
            s.push_str(h);
        }
        s
    }
}
