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
    Fragment,
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
            // WHATWG URL Standard's "path percent-encode set":
            // C0 control percent-encode set (U+0000-U+001F, U+007F), plus space, ", #, <, >, ?, `
            (char_byte <= 0x1F) || (char_byte == 0x7F) || // C0 controls and DEL
            char_byte == b' ' || char_byte == b'"' || char_byte == b'#' ||
            char_byte == b'<' || char_byte == b'>' || char_byte == b'?' ||
            char_byte == b'`'
            // Note: '%' itself should be percent-encoded if it's not part of a valid sequence,
            // but needs_percent_encode typically checks individual bytes before encoding happens.
            // The actual encoding function handles '%' by not double-encoding.
        }
        PercentEncodeSet::Query | PercentEncodeSet::SpecialQuery => {
            // Simplified: Encodes C0 controls, space, #, <, >.
            (char_byte <= 0x1F) || char_byte == 0x7F || // C0 controls and DEL
            char_byte == b' ' || char_byte == b'#' || char_byte == b'<' || char_byte == b'>' || char_byte == b'"'
        }
        PercentEncodeSet::Fragment => {
            // Fragment percent-encode set: C0 controls, space, ", <, >, `
            // U+0000 to U+001F, U+007F
            (char_byte <= 0x1F) || char_byte == 0x7F ||
            char_byte == b' ' || char_byte == b'"' || char_byte == b'<' ||
            char_byte == b'>' || char_byte == b'`'
            // Note: '#' is the fragment delimiter itself, so it wouldn't be part of fragment content
            // unless it was percent-encoded from an earlier stage or part of the final_fragment_to_append.
            // '?' also typically not encoded in fragments unless desired.
        }
    }
}

fn append_percent_encoded(buffer: &mut String, char_byte: u8) {
    buffer.push('%');
    buffer.push_str(&format!("{:02X}", char_byte));
}

// Helper function for path processing and normalization
fn process_and_normalize_path(path_view: &str, is_special: bool, _is_file_scheme_with_empty_host: bool) -> String {
    let mut output_segments: Vec<String> = Vec::new();
    let mut input_path = path_view;

    // Preserve leading slash if present, important for distinguishing absolute/relative paths.
    let mut leading_slash = false;
    if input_path.starts_with('/') || (is_special && input_path.starts_with('\\')) {
        leading_slash = true;
        // input_path = &input_path[1..]; // Temporarily remove for splitting, will add back
    }
    
    // If special, treat backslashes as forward slashes for splitting.
    // This is a bit tricky if path_view itself is temporary.
    // For now, we'll split by both. A more robust way might be to replace \ with / first.
    let segments_iter = input_path.split(|c| c == '/' || (is_special && c == '\\'));

    for segment in segments_iter {
        if segment == ".." {
            // Only pop if there's something to pop and it's not already an effective root ".."
            // (e.g. don't pop if output_segments is empty and no leading slash, or just [""] for leading slash)
            if !output_segments.is_empty() {
                // If the last segment is not empty (not just a marker for a trailing slash from input like "/a//b"), pop it.
                // If it is empty, it means we had something like "/a/", and ".." should pop "a".
                if output_segments.last().map_or(false, |s| s.is_empty()) && output_segments.len() > 1 {
                     output_segments.pop(); // Pop the empty string (trailing slash marker)
                }
                output_segments.pop(); // Pop the actual segment
            } else if leading_slash {
                // Input like "/../foo" - ".." at root does nothing if already at root.
                // If output_segments is empty but there was a leading_slash, it means path started with "/"
                // So "/.." results in just "/"
            }
            // If !leading_slash and output_segments is empty, ".." is added if not file scheme with empty host?
            // For now, ".." at start of relative path remains ".."
            // else if !leading_slash && !is_file_scheme_with_empty_host {
            //     output_segments.push("..".to_string());
            // }

        } else if segment == "." {
            // Do nothing, effectively removing "."
        } else if !segment.is_empty() {
            // Normal segment, percent-encode and add.
            let mut encoded_segment = String::new();
            for char_byte in segment.as_bytes() {
                if needs_percent_encode(*char_byte, PercentEncodeSet::Path) {
                    append_percent_encoded(&mut encoded_segment, *char_byte);
                } else {
                    encoded_segment.push(*char_byte as char);
                }
            }
            output_segments.push(encoded_segment);
        } else if segment.is_empty() && output_segments.is_empty() && !leading_slash {
            // Handles cases like "" or "./" for relative paths where first segment is empty.
            // If path_view was just ".", segment is ".", handled above.
            // If path_view was just "/", segments are ["", ""].
            // If path_view was empty, iterator yields one empty string.
            // output_segments.push("".to_string()); // Keep it to represent it was not just "."
        }
    }
    
    let mut result = String::new();
    if leading_slash {
        result.push('/');
    }

    if !output_segments.is_empty() {
        result.push_str(&output_segments.join("/"));
        // Handle trailing slash if original path_view had one and it wasn't just "/" or "/." etc.
        // or if last segment processed was empty (e.g. from "a//b" or "a/.").
        if (path_view.ends_with('/') || (is_special && path_view.ends_with('\\'))) && !result.ends_with('/') {
            // And it wasn't just input like "/" or "/." or "/.."
            if path_view.len() > 1 && !(path_view.ends_with("/.") || path_view.ends_with("/..")) {
                 result.push('/');
            }
        }
        // If output_segments contained multiple items, and the last one is empty, it means
        // original path ended with something like "a/" or "a//". join("/") would give "a/" or "a//".
        // If original was "/a/b/" -> segments ["", "a", "b", ""], join -> "/a/b/" (leading / added already)
        // If original was "a/b/" -> segments ["a", "b", ""], join -> "a/b/"
        // If original was "a//b" -> segments ["a", "", "b"], join -> "a//b"

        // If result is like "//foo" but should be "/foo" (common after operations like /a/../.. -> "")
        while result.starts_with("//") && result.len() > 1 {
            result.remove(0);
        }
    } else if leading_slash && output_segments.is_empty() {
        // Path was like "/", "/.", "/..". Result is already "/".
    } else {
        // Path was empty, or like ".", or ".." (relative)
        // If path_view was "." or "./", output_segments is empty. Result is "".
        // If path_view was ".." or "../", output_segments might be [".."]. Result is "..".
        // This path is tricky. For now, if output_segments is empty and no leading slash, result is empty.
        // which is fine for inputs like "" or ".".
        // If path_view was ".." it should result in ".."
        if path_view == ".." { return "..".to_string(); }

    }
    // If the result is empty but there was meaningful input that normalized to empty (e.g. "a/.."),
    // and no leading slash, result should be "."
    // This is complex. C++ ada returns "." if input is not empty, scheme is not file, and output is empty.
    // For now, this simplified version might return "" for "a/..".
    // A common behavior: if the original path was not empty and the normalized path is empty,
    // it becomes "." unless it was an absolute path (started with /), then it's "/".
    if result.is_empty() && !leading_slash && !path_view.is_empty() && path_view != "." {
        // e.g. "foo/../" or "foo/bar/../.."
        // result = ".".to_string(); // This is often the desired behavior for relative paths
    }


    result
}


pub fn parse(input: &str, base_url: Option<&UrlAggregator>) -> Result<UrlAggregator, ParseError> {
    // Call the internal parser function
    parse_internal(input, base_url)
}

fn parse_internal(input: &str, _base_url: Option<&UrlAggregator>) -> Result<UrlAggregator, ParseError> {
    let mut state = State::SchemeStart;
    let mut url = UrlAggregator::default();

    let mut input_for_main_parser = input;
    let mut final_fragment_to_append: Option<&str> = None;

    if let Some(hash_pos) = input_for_main_parser.find('#') {
        final_fragment_to_append = Some(&input_for_main_parser[hash_pos + 1..]);
        input_for_main_parser = &input_for_main_parser[..hash_pos];
    }

    // TODO: Perform tab/newline removal and C0 whitespace trim on `input_for_main_parser`.
    // For now, assume these are done.
    let input_data = input_for_main_parser; // Use this for the main parsing loop
    let input_size = input_data.len();      // Length of the string without the final fragment
    let mut input_position = 0;

    // `_fragment` variable from before is replaced by `final_fragment_to_append`

    while input_position <= input_size { 
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
            State::Fragment => {
                // Assumes input_position is at the char *after* '#' (if # was in input_data)
                // or this state might be entered directly if final_fragment_to_append is processed later.
                // For now, this state consumes the rest of `input_data` if a '#' was found mid-string.
                // The final_fragment_to_append logic at the end of parse_internal handles the main fragment.

                // If this state is reached, url.buffer already contains '#' and url.components.hash_start is set.
                
                while input_position < input_size { // input_size is for input_data (pre-pruned fragment)
                    let char_byte = input_data.as_bytes()[input_position];
                    // All characters in the fragment part of input_data are processed here.
                    if needs_percent_encode(char_byte, PercentEncodeSet::Fragment) {
                        append_percent_encoded(&mut url.buffer, char_byte);
                    } else {
                        url.buffer.push(char_byte as char);
                    }
                    input_position += 1;
                }

                // Consumed all of input_data after the '#', if any.
                input_position = input_size + 1; // Terminate main parsing loop.
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

    // TODO: Final validation and setting url.is_valid properly.
    
    if let Some(frag_data) = final_fragment_to_append {
        if url.components.hash_start.is_none() {
             url.components.hash_start = Some(url.buffer.len() as u32);
             url.buffer.push('#');
        }
        // If hash_start is Some, '#' is already in buffer. Append frag_data.
        // The State::Fragment might have already processed some part if '#' was in input_data.
        // This logic appends the *original* fragment string that was pruned.
        // If State::Fragment ran, it means input_data had a '#'. The buffer will be like "base#fragment_from_input_data".
        // We are now appending the true final_fragment_to_append.
        // This means if input was "http://a.com#mid#end",
        // input_data = "http://a.com", final_fragment_to_append = "mid#end"
        // State::Fragment won't run from input_data.
        // url.buffer becomes "http://a.com#", then "mid#end" is appended (encoded).
        // If input was "http://a.com#mid?key=val#end",
        // input_data = "http://a.com#mid?key=val", final_fragment_to_append = "end"
        // State::Fragment will process "mid?key=val" from input_data.
        // url.buffer becomes "http://a.com#mid?key=val" (encoded).
        // Then, "end" is appended here. This seems to double-process or misinterpret.
        
        // Correct logic: State::Fragment processes characters *after* a '#' found in input_for_main_parser.
        // The final_fragment_to_append is *only* from the original input's *first* '#'.
        // So, if State::Fragment ran, it means input_for_main_parser itself contained a '#'.
        // This is usually an error or means the fragment was not correctly pruned.
        // For WHATWG compliance, the first '#' delimits the fragment.
        // Our pruning logic already ensures final_fragment_to_append is from the *first* '#'.
        // input_for_main_parser should NOT contain any '#'. If it does, those should be errors or percent-encoded.
        // So, State::Fragment should ideally not run if pruning is correct.
        // Let's assume for now State::Fragment is for cases where # is not correctly handled by pruning (e.g. future features).
        // The primary way fragment is added is here:
        
        // If hash_start is set, it means '#' is in buffer. We just append frag_data.
        // If hash_start is NOT set, it means no '#' was encountered in the main parsing logic,
        // so we add '#' and then frag_data.

        for &char_byte in frag_data.as_bytes() {
            if needs_percent_encode(char_byte, PercentEncodeSet::Fragment) {
                // The append_percent_encoded function in the prompt is missing the charset argument.
                // Assuming it should be: append_percent_encoded(&mut url.buffer, char_byte);
                // Or if it took charset: append_percent_encoded(&mut url.buffer, char_byte, PercentEncodeSet::Fragment);
                // For now, assuming the simpler version or that it internally knows the fragment rules.
                // The provided function `append_percent_encoded` does not take a charset.
                // Let's assume it's generic or we use the one from the file.
                append_percent_encoded(&mut url.buffer, char_byte);
            } else {
                url.buffer.push(char_byte as char);
            }
        }
    }
    
    url.is_valid = true; // Placeholder
    Ok(url)
}
