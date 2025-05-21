use super::{UrlBase, UrlComponents, SchemeType, HostType};

// Define public enum SetError
#[derive(Debug, PartialEq, Eq)]
pub enum SetError {
    InvalidInput,
}

// Define public struct UrlAggregator
#[derive(Debug, PartialEq, Eq)] // Remove Default here, will implement manually
pub struct UrlAggregator {
    pub buffer: String,
    pub components: UrlComponents,
    is_valid: bool,
    has_opaque_path: bool,
    scheme_type: SchemeType,
    host_type: HostType,
}

impl UrlAggregator {
    // Public constructor
    pub fn new(
        buffer: String,
        components: UrlComponents,
        is_valid: bool,
        has_opaque_path: bool,
        scheme_type: SchemeType,
        host_type: HostType,
    ) -> Self {
        Self {
            buffer,
            components,
            is_valid,
            has_opaque_path,
            scheme_type,
            host_type,
        }
    }

    // Helper function to determine SchemeType from a string slice (without colon)
    // This can be private to the module if not needed elsewhere, or pub if it's a general utility.
    // For now, keeping it internal to where it's used.
    fn determine_scheme_type(scheme_str: &str) -> super::SchemeType {
        match scheme_str {
            "http" => super::SchemeType::Http,
            "https" => super::SchemeType::Https,
            "file" => super::SchemeType::File,
            "ftp" => super::SchemeType::Ftp,
            "ws" => super::SchemeType::Ws,
            "wss" => super::SchemeType::Wss,
            _ => super::SchemeType::NotSpecial,
        }
    }

    pub fn set_scheme_from_slice_with_colon(&mut self, scheme_with_colon_slice: &str) -> Result<(), super::parser::ParseError> {
        // Scheme should be the first part of the URL, so buffer is cleared.
        self.buffer.clear();
        self.buffer.push_str(&scheme_with_colon_slice.to_ascii_lowercase());
        self.components.protocol_end = self.buffer.len() as u32;

        if let Some(colon_pos) = scheme_with_colon_slice.rfind(':') {
            let scheme_part = &scheme_with_colon_slice[..colon_pos];
            self.scheme_type = Self::determine_scheme_type(scheme_part);
        } else {
            // This case should ideally not happen if called with "slice_with_colon"
            // but as a fallback:
            self.scheme_type = Self::determine_scheme_type(scheme_with_colon_slice);
        }
        // TODO: Add validation if scheme_part is empty or invalid beyond type lookup
        Ok(())
    }

    // Setter methods for components
    // These methods will append to self.buffer and update self.components
    // For now, percent encoding is a TODO.

    pub fn set_username(&mut self, username_slice: &str, _charset: super::parser::UserInfoCharSet) -> Result<(), super::parser::ParseError> {
        // TODO: Implement proper percent encoding for username_slice based on charset.
        // For now, append raw and update relevant component indices.
        // Assume username is appended after protocol, before any password or host.
        // If buffer is "http://", username "user" -> "http://user"
        
        let start_of_username = self.components.protocol_end; // Or some other logic if buffer isn't just scheme yet
        // This assumes buffer is currently holding up to protocol_end or similar start point for authority.
        // If there's existing userinfo/host, this logic needs to be much more complex (replace/insert).
        // For initial parsing, we build sequentially.

        // Username is appended after elements like "scheme://"
        // The parser ensures the buffer is truncated to the correct point (e.g. after "//")
        // self.components.host_start should be set by the parser to the start of where username is written.
        self.buffer.truncate(self.components.host_start as usize); // Truncate to where username should begin
        
        self.buffer.push_str(username_slice);
        self.components.username_end = self.buffer.len() as u32;
        // After username, password or host might follow. For now, mark host_start after username.
        // The parser State::Authority will manage adding ':' for password or '@' for host.
        // Let password_end be represented by where the host would start if no password.
        // If a password is set, it will update this.
        // self.components.host_start = self.components.username_end; // This will be updated by parser if password/host follows
        self.components.host_end = self.components.username_end;   // and ends there too, until host is set.
        Ok(())
    }

    pub fn set_password(&mut self, password_slice: &str, _charset: super::parser::UserInfoCharSet) -> Result<(), super::parser::ParseError> {
        // Password is appended after "username:"
        // The parser ensures the buffer is truncated to the correct point (e.g. after "username:")
        // self.components.username_end should point to the end of "username" (or "username:")
        // The parser's Authority state should have appended ':' before calling this.
        // So, we truncate up to the point after the ':' separating username and password.
        // This implies username_end might need to be more nuanced or parser manages buffer for the ':'
        // Let's assume parser has already added ':', and username_end is *after* this ':'.
        // Or, more simply, parser truncates to after username, adds ':', then calls set_password.
        // For now, assume buffer is at "scheme://username:"
        // truncate up to current buffer end, which is where password should start
        // self.buffer.truncate(self.components.username_end as usize); // This should be after the ':' if parser added it.

        self.buffer.push_str(password_slice);
        // After password, the '@' and host follow.
        // host_start will be set by the parser after it appends '@'.
        // For now, mark that password ends here.
        // Let's use username_end to mark end of "username:password" or "username"
        // The parser will use this to know where to put '@'
        self.components.username_end = self.buffer.len() as u32; // username_end now marks end of password
        // self.components.host_start = self.buffer.len() as u32; // Parser will set this after adding '@'
        self.components.host_end = self.buffer.len() as u32;
        Ok(())
    }

    pub fn set_host_from_slice(&mut self, host_slice: &str, _is_special_url: bool) -> Result<(), super::parser::ParseError> {
        // Host is appended after "scheme://" or "scheme://user:pass@"
        // The parser ensures the buffer is truncated to the correct point (e.g. after "//" or "@")
        // self.components.host_start should be set by the parser to this point.
        self.buffer.truncate(self.components.host_start as usize);

        let processed_host = host_slice.to_ascii_lowercase();
        if processed_host.is_empty() && self.is_special() {
            // For special schemes, an empty host is not allowed after authority markers like "//"
            // This check might be better placed in the parser's Host state.
            // For now, if it's special and host_slice is empty, this is problematic if host_start implies authority.
            // However, `file:///path/to/file` results in an empty host. `set_host_from_slice("")` is called.
            // The file state needs to handle this. If host_slice is empty, host_type remains Default/Unknown.
            // No change to buffer, host_end remains same as host_start.
            self.components.host_end = self.components.host_start;
        } else {
            self.buffer.push_str(&processed_host);
            self.components.host_end = self.buffer.len() as u32;
        }
        self.components.host_end = self.buffer.len() as u32;
        
        // Placeholder for host_type (assuming it's not an IP for now)
        if processed_host.starts_with('[') && processed_host.ends_with(']') {
            self.host_type = super::HostType::Ipv6; // Basic check
        } else {
            // TODO: Check for IPv4
            self.host_type = super::HostType::Default;
        }
        Ok(())
    }

    pub fn set_port_from_slice(&mut self, port_digits_slice: &str, _is_special_url: bool) -> Result<(), super::parser::ParseError> {
        // Port is appended after the host.
        // Buffer: "http://example.com", port_digits "8080" -> "http://example.com:8080"
        self.buffer.truncate(self.components.host_end as usize); // Ensure we are at end of host, removing old port if any

        if port_digits_slice.is_empty() && self.is_special() {
             // Special schemes effectively have a default port if empty after colon,
             // but setting it to empty here might mean "remove port".
             // C++ ada seems to set port to `SCHEME_DEFAULT_PORT` if empty and special.
             // For now, if empty, we effectively remove/don't set a port.
            self.components.port_value = None;
            // The buffer does not get a ':'
            return Ok(());
        }
        
        match port_digits_slice.parse::<u16>() {
            Ok(port_val) => {
                // TODO: Add validation based on scheme's default port if port_val matches it (then maybe store None or special value)
                // C++ ada: if (parsed_port == scheme_default_port()) { components.port = omitted; }
                // For now, always store the parsed value.
                self.components.port_value = Some(port_val);
                self.buffer.push(':');
                self.buffer.push_str(port_digits_slice); // Append the original digits
                self.components.host_end = self.buffer.len() as u32; // Update host_end to include the port string
            }
            Err(_) => {
                // TODO: Set validation error flag on UrlAggregator if this happens.
                // self.is_valid = false; (or similar)
                return Err(super::parser::ParseError::InvalidPort);
            }
        }
        Ok(())
    }

    pub fn ensure_pathname_starts_with_slash_if_authority(&mut self) {
        // Check if authority is present. Authority ends at host_end (which includes port if present).
        // Protocol_end points after "scheme:".
        // If host_end is greater than protocol_end, it implies some form of authority was parsed.
        // (This is a simplification; host_start would be a better indicator if it's set after protocol_end)
        let authority_present = self.components.host_end > self.components.protocol_end && // Basic check for host/port
                                (self.components.host_start > self.components.protocol_end || // Userinfo or host was set
                                 self.buffer[self.components.protocol_end as usize ..].starts_with("//")); // Common case for special URLs

        if authority_present {
            // If pathname_start is already set and points to a slash, or if buffer at host_end starts with slash, it's fine.
            // Pathname_start should be at or after host_end.
            if self.components.pathname_start >= self.components.host_end { // Path is after authority
                if (self.components.pathname_start as usize) == self.buffer.len() && !self.buffer.ends_with('/') {
                    // Path is about to start right after authority, and no slash is there.
                    self.buffer.push('/');
                    self.components.pathname_start = self.buffer.len() as u32 -1; // Path starts with this slash
                } else if (self.components.pathname_start as usize) < self.buffer.len() && self.buffer.as_bytes()[self.components.pathname_start as usize] != b'/' {
                    // Pathname_start is set, but it's not a slash. This is unusual if authority was present.
                    // This case might indicate an issue or require inserting a slash.
                    // For now, assume if pathname_start is set, it's correctly pointing to the start of path (or a slash).
                } else if (self.components.pathname_start as usize) == self.buffer.len() && self.buffer.ends_with('/') {
                    // Already ends with a slash, pathname_start is at its beginning.
                    self.components.pathname_start = self.buffer.len() as u32 -1;
                }
            } else { // Pathname_start is not yet set meaningfully after authority
                 if self.buffer.len() == self.components.host_end as usize && !self.buffer.ends_with('/') {
                    self.buffer.push('/');
                 }
                 self.components.pathname_start = self.buffer.len() as u32 -1; // Path starts with this slash
            }
        }
        // If no authority, path can start immediately after scheme (e.g. mailto:user) or with a slash (e.g. file:/path)
        // This method only cares about enforcing the slash *if authority was present*.
    }

    pub fn set_pathname_start_if_not_set(&mut self) {
        // This function might be too simplistic. pathname_start is usually set by previous states.
        // If it's 0 and protocol_end is also 0 (e.g. relative path parsing), then path starts at 0.
        // If protocol_end is set, it's after that. If host_end is set, it's after that.
        // Let's assume it means "if pathname_start is not yet advanced beyond where host/scheme ended"
        if self.components.pathname_start <= self.components.host_end && self.components.host_end > 0 {
             self.components.pathname_start = self.components.host_end;
        } else if self.components.pathname_start <= self.components.protocol_end && self.components.protocol_end > 0 {
             self.components.pathname_start = self.components.protocol_end;
        } else if self.components.pathname_start == 0 && self.components.protocol_end == 0 && self.components.host_start == 0 && self.components.host_end == 0 {
            // Default case for paths like "foo/bar"
            self.components.pathname_start = 0;
        }
        // Ensure buffer is at least as long as pathname_start if we are about to append to path
        if (self.buffer.len() as u32) < self.components.pathname_start {
            // This would be an inconsistent state.
            // For safety, set pathname_start to current buffer end if it's supposed to be further.
             self.components.pathname_start = self.buffer.len() as u32;
        }
    }

    // Getters
    pub fn href(&self) -> &str {
        &self.buffer
    }

    pub fn protocol(&self) -> &str {
        if self.components.protocol_end > 0 && (self.components.protocol_end as usize) <= self.buffer.len() {
            &self.buffer[0..self.components.protocol_end as usize]
        } else {
            ""
        }
    }

    pub fn username(&self) -> &str {
        // Placeholder
        ""
    }

    pub fn password(&self) -> &str {
        // Placeholder
        ""
    }

    pub fn host(&self) -> &str {
        // Placeholder
        ""
    }

    pub fn hostname(&self) -> &str {
        // Placeholder
        ""
    }

    pub fn port(&self) -> &str {
        // Placeholder
        ""
    }

    pub fn pathname(&self) -> &str {
        // Placeholder
        ""
    }

    pub fn search(&self) -> &str {
        // Placeholder
        ""
    }

    pub fn hash(&self) -> &str {
        // Placeholder
        ""
    }

    // Setters
    pub fn set_href(&mut self, input: &str) -> Result<(), SetError> {
        self.buffer = input.to_string();
        // Potentially invalidate or reset self.components here
        // For now, just reset to default as a basic measure
        self.components = UrlComponents::default();
        self.is_valid = false; // Mark as potentially invalid until re-parsed/validated
        Ok(())
    }

    pub fn set_protocol(&mut self, _input: &str) -> Result<(), SetError> {
        // Placeholder
        Err(SetError::InvalidInput) // Or Ok(()) if we want to allow setting for now
    }

    pub fn set_username(&mut self, _input: &str) -> Result<(), SetError> {
        // Placeholder
        Err(SetError::InvalidInput)
    }

    pub fn set_password(&mut self, _input: &str) -> Result<(), SetError> {
        // Placeholder
        Err(SetError::InvalidInput)
    }

    pub fn set_host(&mut self, _input: &str) -> Result<(), SetError> {
        // Placeholder
        Err(SetError::InvalidInput)
    }

    pub fn set_hostname(&mut self, _input: &str) -> Result<(), SetError> {
        // Placeholder
        Err(SetError::InvalidInput)
    }

    pub fn set_port(&mut self, _input: &str) -> Result<(), SetError> {
        // Placeholder
        Err(SetError::InvalidInput)
    }

    pub fn set_pathname(&mut self, _input: &str) -> Result<(), SetError> {
        // Placeholder
        Err(SetError::InvalidInput)
    }

    pub fn set_search(&mut self, _input: &str) -> Result<(), SetError> {
        // Placeholder
        Err(SetError::InvalidInput)
    }

    pub fn set_hash(&mut self, _input: &str) -> Result<(), SetError> {
        // Placeholder
        Err(SetError::InvalidInput)
    }
}

// Manual implementation of Default for UrlAggregator
impl Default for UrlAggregator {
    fn default() -> Self {
        Self {
            buffer: String::new(),
            components: UrlComponents::default(), // UrlComponents should also derive Default
            is_valid: true, // Default assumption, parser will invalidate if needed
            has_opaque_path: false,
            scheme_type: SchemeType::NotSpecial,
            host_type: HostType::Default,
        }
    }
}

impl UrlBase for UrlAggregator {
    fn is_valid(&self) -> bool {
        self.is_valid
    }

    fn has_opaque_path(&self) -> bool {
        self.has_opaque_path
    }

    fn scheme_type(&self) -> SchemeType {
        // Return a copy of the scheme_type
        self.scheme_type
    }

    fn host_type(&self) -> HostType {
        // Return a copy of the host_type
        self.host_type
    }

    fn is_special(&self) -> bool {
        match self.scheme_type {
            SchemeType::Http | SchemeType::Https | SchemeType::File | SchemeType::Ftp | SchemeType::Ws | SchemeType::Wss => true,
            SchemeType::NotSpecial => false,
        }
    }

    fn origin(&self) -> String {
        // For now, return an empty string or a placeholder like "todo_origin()"
        "todo_origin()".to_string()
    }

    fn protocol_end(&self) -> u32 {
        self.components.protocol_end
    }

    fn username_end(&self) -> u32 {
        self.components.username_end
    }

    fn host_start(&self) -> u32 {
        self.components.host_start
    }

    fn host_end(&self) -> u32 {
        self.components.host_end
    }

    fn port(&self) -> Option<u16> {
        self.components.port
    }

    fn pathname_start(&self) -> u32 {
        self.components.pathname_start
    }

    fn search_start(&self) -> Option<u32> {
        self.components.search_start
    }

    fn hash_start(&self) -> Option<u32> {
        self.components.hash_start
    }

    fn to_legacy_string(&self) -> String {
        // For now, return self.buffer.clone(), or a placeholder
        self.buffer.clone()
    }
}
