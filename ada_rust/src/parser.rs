use crate::url_aggregator::UrlAggregator;
use super::{UrlComponents, SchemeType, HostType}; // Import from parent module

#[derive(Debug)]
pub enum ParseError {
    InvalidUrl,
    InvalidBaseUrl,
    UnexpectedToken,
    InvalidPort, // Added for port parsing errors
}

// Placeholder for character set for percent-encoding userinfo
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserInfoCharSet {
    Default, // Represents USERINFO_PERCENT_ENCODE for now
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    SchemeStart,
    Scheme,
    NoScheme,
    SpecialRelativeOrAuthority,
    PathOrAuthority,
    RelativeScheme,
    RelativeSlash,
    SpecialAuthoritySlashes,
    SpecialAuthorityIgnoreSlashes,
    Authority,
    Host,
    Port,
    File,
    FileSlash,
    FileHost,
    PathStart,
    Path,
    OpaquePath,
    Query,
    Fragment, // Added Fragment state
}

// Character sets for percent-encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PercentEncodeSet {
    // UserInfo,
    Path,
    C0Control,
    Query,
    SpecialQuery,
    // Fragment can be added if needed, though often it has a very restricted set or is handled differently.
}

// Updated percent-encoding function
fn needs_percent_encode(char_byte: u8, charset: PercentEncodeSet) -> bool {
    match charset {
        PercentEncodeSet::C0Control => {
            // Encode U+0000 to U+001F (C0 controls) and U+007F (DELETE)
            // Also encode space ' ', and forbidden opaque path chars like '#', '?'
            (char_byte <= 0x1F) || char_byte == 0x7F || char_byte == b' ' || char_byte == b'#' || char_byte == b'?'
        }
        PercentEncodeSet::Path => {
            // Placeholder: Encodes C0 controls, space, ", #, <, >, ?, `
            (char_byte <= 0x1F) || char_byte == 0x7F || char_byte == b' ' ||
            char_byte == b'"' || char_byte == b'#' || char_byte == b'<' || char_byte == b'>' ||
            char_byte == b'?' || char_byte == b'`'
            // TODO: Add other characters as per PATH_PERCENT_ENCODE set from WHATWG spec.
        }
        PercentEncodeSet::Query | PercentEncodeSet::SpecialQuery => {
            // Simplified: Encodes C0 controls, space, #, <, >.
            // Query encoding in spec is complex: application/x-www-form-urlencoded (space to '+') vs. general query.
            // WHATWG URL spec's "query state" percent-encode set includes C0 controls, space, #, <, >.
            // SpecialQuery might be slightly more permissive for ' / ? ; = &' but for now, same as Query.
            // It should NOT encode '+' if space is being encoded as '+'.
            // If space is %20, then '+' can be encoded if it's not meant as a delimiter.
            // For now, let's assume space is %20, and we are not encoding typical query delimiters like '&', '=', '+'.
            (char_byte <= 0x1F) || char_byte == 0x7F || // C0 controls and DEL
            char_byte == b' ' || char_byte == b'#' || char_byte == b'<' || char_byte == b'>' || char_byte == b'"'
            // Does NOT encode: & = + ; / ? (unless part of a very specific set for special URLs)
            // The task asks for Query to encode C0, space, ", #, <, >. This is implemented.
            // SpecialQuery is same for now.
        }
    }
}

fn append_percent_encoded(buffer: &mut String, char_byte: u8) {
    buffer.push('%');
    buffer.push_str(&format!("{:02X}", char_byte));
}


pub fn parse(input: &str, base_url: Option<&UrlAggregator>) -> Result<UrlAggregator, ParseError> {
    // Call the internal parser function
    parse_internal(input, base_url)
}

fn parse_internal(input: &str, _base_url: Option<&UrlAggregator>) -> Result<UrlAggregator, ParseError> {
    let mut state = State::SchemeStart;
    let mut url = UrlAggregator::default(); // Requires Default trait for UrlAggregator

    // Pre-processing
    let mut input_data = input;
    // TODO: handle tabs/newlines
    // TODO: handle C0 whitespace trim
    let _fragment: Option<&str> = None; // TODO: implement prune_hash

    let mut input_position = 0;
    let input_size = input_data.len();

    while input_position <= input_size { // Condition might need adjustment for Rust iterators vs C++ pointer logic
        match state {
            State::SchemeStart => {
                if input_position < input_size && input_data.as_bytes()[input_position].is_ascii_alphabetic() {
                    // url.buffer.push(input_data.as_bytes()[input_position].to_ascii_lowercase()); // Simplified, actual component setting is more complex
                    state = State::Scheme;
                    input_position += 1;
                } else {
                    state = State::NoScheme;
                    // No decrement of input_position here, as NO_SCHEME will handle it or expect current position.
                }
            }
            State::Scheme => {
                let start = input_position - 1; // Scheme data started one char back
                let initial_buffer_len_for_scheme = url.buffer.len();

                while input_position < input_size {
                    let current_byte = input_data.as_bytes()[input_position];
                    if current_byte.is_ascii_alphanumeric() || current_byte == b'+' || current_byte == b'-' || current_byte == b'.' {
                        input_position += 1;
                    } else if current_byte == b':' {
                        let scheme_with_colon_slice = &input_data[start..input_position+1];
                        // Call the new method on UrlAggregator
                        match url.set_scheme_from_slice_with_colon(scheme_with_colon_slice) {
                            Ok(_) => {
                                // Scheme successfully set. Now determine transitions.
                                let current_scheme_type = url.scheme_type(); // scheme_type() is from UrlBase trait

                                if current_scheme_type == super::SchemeType::File {
                                    state = State::File;
                                } else if url.is_special() && _base_url.is_some() && _base_url.unwrap().scheme_type() == current_scheme_type {
                                    // TODO: The base_url handling needs the base_url to be parsed first.
                                    // This implies base_url.unwrap().is_special() is also true.
                                    state = State::SpecialRelativeOrAuthority;
                                } else if url.is_special() {
                                    state = State::SpecialAuthoritySlashes;
                                } else if input_position + 1 < input_size && input_data.as_bytes()[input_position + 1] == b'/' {
                                    // Check character *after* the colon.
                                    state = State::PathOrAuthority;
                                    // Note: PathOrAuthority will expect to be on the first '/' *after* the colon.
                                    // The main loop's structure needs to ensure input_position is advanced past the colon.
                                } else {
                                    // url.clear_pathname(); // TODO: Implement this method
                                    url.has_opaque_path = true;
                                    url.components.pathname_start = url.buffer.len() as u32; // Path starts after scheme colon
                                    state = State::OpaquePath;
                                }
                            }
                            Err(e) => { // Handle error from set_scheme_from_slice_with_colon if it can fail
                                // This might indicate an invalid scheme character not caught by the loop,
                                // or other internal error. For now, treat as fatal parse error.
                                return Err(e);
                            }
                        }
                        input_position += 1; // Consume the ':'
                        continue; // Restart loop with new state and position after ':'
                    } else {
                        // Invalid character in scheme, break from this loop to go to NoScheme.
                        break;
                    }
                }

                // If loop finished without finding ':', it's not a valid scheme with colon.
                // Reset and go to NoScheme.
                // url.buffer.truncate(initial_buffer_len_for_scheme); // Restore buffer to state before scheme attempt
                url.buffer.clear(); // Clearing buffer as scheme is first.
                url.components = UrlComponents::default();
                url.scheme_type = SchemeType::NotSpecial;

                state = State::NoScheme;
                input_position = 0; // Reset pointer to start over for NoScheme processing.
                continue; // Restart loop for NoScheme.
            }
            State::NoScheme => {
                // If base_url is None or has_opaque_path, or is file scheme and current input is not windows drive letter...
                // return Err(ParseError::InvalidUrl) or some specific error.
                // For now, placeholder:
                // TODO: Implement NoScheme logic properly
                state = State::PathStart; // Temporary: treat as path if no scheme
                // No input_position change here, PathStart will handle it.
                continue;
            }
            State::SpecialRelativeOrAuthority => { /* TODO */ state = State::Host; /* temporary transition */ input_position = input_size + 1; /* break loop */ continue; }
            State::PathOrAuthority => {
                // Entered after a non-special scheme, e.g., "mailto:foo" (input_position at 'f')
                // or "foo:/bar" (input_position at '/')
                if input_position < input_size && input_data.as_bytes()[input_position] == b'/' {
                    state = State::Authority;
                    input_position +=1; // Consume the first '/'
                } else {
                    // Not starting with '/', so it's a path after non-special scheme
                    // e.g. "mailto:foo" -> current char is 'f'
                    // url.clear_pathname(); // TODO
                    url.has_opaque_path = false; // It's not an opaque path
                    state = State::Path;
                    // Do not consume here, Path state will handle it.
                }
                continue;
            }
            State::RelativeScheme => { /* TODO */ state = State::Host; /* temporary transition */ input_position = input_size + 1; /* break loop */ continue; }
            State::RelativeSlash => { /* TODO */ state = State::Host; /* temporary transition */ input_position = input_size + 1; /* break loop */ continue; }
            State::SpecialAuthoritySlashes => {
                // Expects to be on the char *after* scheme's colon.
                // e.g. for "http://", this state is after the ':'
                if input_position < input_size && input_data.as_bytes()[input_position] == b'/' &&
                   input_position + 1 < input_size && input_data.as_bytes()[input_position + 1] == b'/' {
                    state = State::SpecialAuthorityIgnoreSlashes;
                    input_position += 2; // Consume both slashes
                } else {
                    // According to WHATWG, this is a validation error and should proceed to SpecialAuthorityIgnoreSlashes.
                    // TODO: Set validation error flag.
                    state = State::SpecialAuthorityIgnoreSlashes;
                    // Do not consume here, SpecialAuthorityIgnoreSlashes handles it.
                }
                continue;
            }
            State::SpecialAuthorityIgnoreSlashes => {
                // Skip any leading slashes.
                while input_position < input_size && input_data.as_bytes()[input_position] == b'/' {
                    // TODO: Set validation error flag ("unexpected / before host")
                    input_position += 1;
                }
                state = State::Authority;
                // Do not consume here, Authority handles current char.
                continue;
            }
            State::Authority => {
                // At the start of authority component (e.g., after "//" or after first "/" in "file:/c:/...")
                // Find end of authority (before '/', '?', '#' or EOF)
                let mut end_of_authority = input_position;
                while end_of_authority < input_size {
                    let byte = input_data.as_bytes()[end_of_authority];
                    if byte == b'/' || byte == b'?' || byte == b'#' || (url.is_special() && byte == b'\\') {
                        break;
                    }
                    end_of_authority += 1;
                }
                
                let authority_data_view = &input_data[input_position..end_of_authority];
                let mut host_view_start_in_authority = 0; // Start of host part within authority_data_view

                // Pragmatic approach: find the last '@'
                if let Some(last_at_pos) = authority_data_view.rfind('@') {
                    let userinfo_view = &authority_data_view[..last_at_pos];
                    host_view_start_in_authority = last_at_pos + 1;

                    // TODO: More complex userinfo parsing (multiple '@', percent encoding)
                    // For now, split by first ':', if any, to get username/password
                    let mut username_part = userinfo_view;
                    let mut password_part = "";

                    if let Some(colon_pos) = userinfo_view.find(':') {
                        username_part = &userinfo_view[..colon_pos];
                        password_part = &userinfo_view[colon_pos+1..];
                    }
                    
                    // Call UrlAggregator setters
                    // These clear previous authority parts and append.
                    if !username_part.is_empty() { // C++ ada allows empty username with password
                        // TODO: use actual UserInfoCharSet::Default
                        url.set_username(username_part, UserInfoCharSet::Default).map_err(|e| e)?; 
                    }
                    if !password_part.is_empty() || (username_part.is_empty() && !userinfo_view.is_empty() && userinfo_view.contains(':')) {
                         // Add the ':' that separates username and password to the buffer
                        if url.components.username_end > url.components.protocol_end && url.components.host_start == url.components.username_end {
                             url.buffer.insert(url.components.username_end as usize, ':');
                             url.components.username_end +=1; // Adjust if username_end is just after username content
                             url.components.host_start +=1; // host_start was same as username_end, so shift it too
                        }
                        url.set_password(password_part, UserInfoCharSet::Default).map_err(|e| e)?;
                    }
                    // Add the '@' to the buffer
                    url.buffer.push('@');
                    url.components.host_start = url.buffer.len() as u32; // Host starts after the final '@'
                }
                // Else (no '@'), the whole authority_data_view is for the host.
                // url.components.host_start is already at the beginning of where host should be (after scheme, or after userinfo if it was set)

                // Advance global input_position to where the host data begins (relative to input_data)
                input_position += host_view_start_in_authority;
                
                state = State::Host;
                // Do not consume here. State::Host will process from current input_position.
                continue;
            }
            State::Host => {
                // input_position is at the start of the host data.
                // Host data ends at ':', '/', '?', '#', or EOF. Or '\' for special URLs.
                let mut host_end_offset = 0;
                while input_position + host_end_offset < input_size {
                    let byte = input_data.as_bytes()[input_position + host_end_offset];
                    if byte == b':' || byte == b'/' || byte == b'?' || byte == b'#' || (url.is_special() && byte == b'\\') {
                        break;
                    }
                    host_end_offset += 1;
                }

                let host_view = &input_data[input_position .. input_position + host_end_offset];
                
                // TODO: If host_view is empty and url is special, validation error.
                // C++ ada: if (url.is_special() and host_view.empty()) { return set_valid(false); }
                if url.is_special() && host_view.is_empty() {
                    // TODO: set validation error on url
                    return Err(ParseError::InvalidUrl); // Or a more specific error
                }

                url.set_host_from_slice(host_view, url.is_special()).map_err(|e|e)?;
                input_position += host_end_offset; // Consume host part

                // Check character that terminated host
                if input_position < input_size && input_data.as_bytes()[input_position] == b':' {
                    // TODO: if url.is_cannot_be_a_base_url(), then error.
                    // C++: if (url.is_cannot_be_a_base_url()) { return set_valid(false); }
                    state = State::Port;
                    input_position += 1; // Consume the ':' before Port state processes digits.
                } else {
                    state = State::PathStart;
                    // Do not consume, PathStart will handle current char (e.g. '/', '?', '#')
                }
                continue;
            }
            State::Port => {
                // input_position is at the start of port digits (char after ':')
                let mut port_end_offset = 0;
                while input_position + port_end_offset < input_size {
                    if !input_data.as_bytes()[input_position + port_end_offset].is_ascii_digit() {
                        break;
                    }
                    port_end_offset += 1;
                }
                
                let port_view = &input_data[input_position .. input_position + port_end_offset];
                url.set_port_from_slice(port_view, url.is_special()).map_err(|e| e)?;
                
                input_position += port_end_offset; // Consume port digits

                // TODO: Validate port based on is_special() and current char (EOF, '/', '?', '#', '\')
                // C++ ada: if (is_eof() or is_path_start_char(byte_at_offset(0))) ...
                // For now, assume valid if set_port_from_slice didn't error.
                
                state = State::PathStart;
                // Do not consume, PathStart handles current char.
                continue;
            }
            State::File => { /* TODO */ state = State::Host; /* temporary transition */ input_position = input_size + 1; /* break loop */ continue; }
            State::FileSlash => { /* TODO */ state = State::Host; /* temporary transition */ input_position = input_size + 1; /* break loop */ continue; }
            State::FileHost => { /* TODO */ state = State::Host; /* temporary transition */ input_position = input_size + 1; /* break loop */ continue; }
            State::PathStart => {
                // TODO: Handle path start logic, including special URL backslash.
                // For now, assume it's a regular path.
                if url.is_special() {
                    // C++: if (url.is_special() && is_path_start_char_special_url(byte_at_offset(0)))
                    // For now, simplify: if special, it might need to normalize slashes.
                }
                state = State::Path;
                // Do not consume, Path state will handle current char.
                continue;
            }
            State::Path => { 
                // TODO: Implement path parsing (consume until ?, #, or EOF)
                // url.components.pathname_start should be set correctly.
                // For now, consume all remaining.
                let mut path_end_offset = 0;
                while input_position + path_end_offset < input_size {
                    let byte = input_data.as_bytes()[input_position + path_end_offset];
                    if byte == b'?' || byte == b'#' { // Stop before query or hash
                        break;
                    }
                    // TODO: Handle percent encoding, normalization (slashes, dots)
                    path_end_offset += 1;
                }
                let path_view = &input_data[input_position .. input_position + path_end_offset];
                // url.set_pathname_from_slice(path_view); // TODO: Need this method on UrlAggregator
                url.buffer.push_str(path_view); // Simplistic append to buffer
                url.components.pathname_start = url.components.host_end; // Path starts after host/port
                // If there was userinfo but no host, pathname_start needs to be after userinfo.
                // This needs careful management of indices.
                // For now:
                if url.components.host_end == 0 && url.components.host_start > 0 { // Likely userinfo only
                    url.components.pathname_start = url.components.host_start;
                } else if url.components.host_end == 0 && url.components.username_end > 0 {
                     url.components.pathname_start = url.components.username_end;
                } else if url.components.host_end == 0 && url.components.protocol_end > 0 {
                     url.components.pathname_start = url.components.protocol_end;
                }


                input_position += path_end_offset;
                
                // After path, check for query or hash
                if input_position < input_size && input_data.as_bytes()[input_position] == b'?' {
                    state = State::Query;
                    // input_position += 1; // Query state will consume '?'
                } else if input_position < input_size && input_data.as_bytes()[input_position] == b'#' {
                    // state = State::Fragment (or handle fragment pre-parsing)
                    // For now, just end.
                    input_position = input_size + 1; // Break loop
                } else {
                    input_position = input_size + 1; // Break loop, EOF
                }
                continue;
            }
            State::OpaquePath => {
                // url.components.pathname_start should already be set to url.buffer.len() by State::Scheme
                // url.has_opaque_path should already be true.
                // Buffer currently contains "scheme:"

                let start_of_opaque_path_in_buffer = url.components.pathname_start;
                // Ensure buffer is truncated to this point before appending, in case of re-entry or complex scenarios.
                url.buffer.truncate(start_of_opaque_path_in_buffer as usize);

                while input_position < input_size {
                    let char_byte = input_data.as_bytes()[input_position];

                    if char_byte == b'?' {
                        url.components.search_start = Some(url.buffer.len() as u32);
                        url.buffer.push('?');
                        input_position += 1;
                        state = State::Query;
                        break; // Break from OpaquePath loop, continue in main loop
                    } else if char_byte == b'#' {
                        url.components.hash_start = Some(url.buffer.len() as u32);
                        url.buffer.push('#');
                        input_position += 1;
                        state = State::Fragment;
                        break; // Break from OpaquePath loop, continue in main loop
                    } else {
                        // Percent-encode C0 controls, space, #, ?
                        if needs_percent_encode(char_byte, PercentEncodeSet::C0Control) {
                            append_percent_encoded(&mut url.buffer, char_byte);
                        } else {
                            url.buffer.push(char_byte as char);
                        }
                        input_position += 1;
                    }
                }

                if state == State::OpaquePath { // Loop finished due to EOF
                    input_position = input_size + 1; // Terminate parsing
                }
                // If state changed to Query or Fragment, the main loop will continue with that state.
                continue;
            }
            State::Query => {
                // Assumes input_position is at the char *after* '?'
                // url.components.search_start points to the '?' in url.buffer
                // url.buffer currently ends with '?'

                let chosen_charset = if url.is_special() {
                    PercentEncodeSet::SpecialQuery
                } else {
                    PercentEncodeSet::Query
                };

                while input_position < input_size {
                    let char_byte = input_data.as_bytes()[input_position];

                    if char_byte == b'#' {
                        url.components.hash_start = Some(url.buffer.len() as u32);
                        url.buffer.push('#');
                        input_position += 1;
                        state = State::Fragment;
                        break; // Break from Query loop
                    } else {
                        if needs_percent_encode(char_byte, chosen_charset) {
                            append_percent_encoded(&mut url.buffer, char_byte);
                        } else {
                            url.buffer.push(char_byte as char);
                        }
                        input_position += 1;
                    }
                }

                if state == State::Query { // Loop finished due to EOF
                    input_position = input_size + 1; // Terminate parsing
                }
                // If state changed to Fragment, main loop continues with that.
                // If loop finished by EOF, main loop terminates.
                continue;
            }
        }

        // If a state did not `continue`, `return`, or explicitly set `input_position` to `input_size + 1` to break,
        // and `input_position` is still within bounds, it implies an unhandled situation or a state that should have consumed a character.
        // Most states that transition internally or consume a character should `continue`.
        // States that reach a final point or an error should either return or break the loop by setting `input_position > input_size`.
        // The `while input_position <= input_size` condition means the loop continues if `input_position == input_size` (on the EOF pseudo-char).
        // If `input_position` naturally increments to `input_size + 1`, the loop terminates.
        if input_position > input_size { // Ensure loop terminates if input_position is pushed beyond input_size by a state.
            break;
        }
    }

    // TODO: handle fragment (usually pre-parsed and appended at the end)
    // TODO: Final validation and setting url.is_valid properly.
    url.is_valid = true; // Placeholder

    Ok(url)
}
