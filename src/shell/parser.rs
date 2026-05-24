use std::collections::HashMap;

pub fn parse_input(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single_quotes = false;
    let mut in_double_quotes = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\\' && in_double_quotes {
            if i + 1 < chars.len() {
                let next = chars[i + 1];
                match next {
                    '"' | '\\' => {
                        current.push(next);
                        i += 2;
                        continue;
                    }
                    _ => {
                        current.push('\\');
                        current.push(next);
                        i += 2;
                        continue;
                    }
                }
            }
        }

        match ch {
            '\\' if !in_single_quotes && !in_double_quotes => {
                i += 1;
                if i < chars.len() {
                    current.push(chars[i]);
                }
            }
            '\'' if !in_double_quotes => {
                in_single_quotes = !in_single_quotes;
            }
            '"' if !in_single_quotes => {
                in_double_quotes = !in_double_quotes;
            }
            ' ' | '\t' if !in_single_quotes && !in_double_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            '1' | '2'
                if !in_single_quotes
                    && !in_double_quotes
                    && i + 1 < chars.len()
                    && chars[i + 1] == '>' =>
            {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
                let mut token = String::new();
                token.push(ch);
                token.push('>');
                i += 2;
                if i < chars.len() && chars[i] == '>' {
                    token.push('>');
                } else {
                    i -= 1;
                }
                args.push(token);
            }
            '|' if !in_single_quotes && !in_double_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
                args.push("|".to_string());
            }
            '>' if !in_single_quotes && !in_double_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
                let mut token = String::from(">");
                if i + 1 < chars.len() && chars[i + 1] == '>' {
                    token.push('>');
                    i += 1;
                }
                args.push(token);
            }
            _ => {
                current.push(ch);
            }
        }

        i += 1;
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

pub fn expand_variables(s: &str, variables: &HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            match chars.peek() {
                Some(&'{') => {
                    chars.next();
                    let mut name = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            chars.next();
                            break;
                        }
                        name.push(chars.next().unwrap());
                    }
                    let value = variables.get(&name).map(|s| s.as_str()).unwrap_or("");
                    result.push_str(value);
                }
                Some(&next) if next.is_ascii_alphabetic() || next == '_' => {
                    let mut name = String::new();
                    name.push(chars.next().unwrap());
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_alphanumeric() || c == '_' {
                            name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    let value = variables.get(&name).map(|s| s.as_str()).unwrap_or("");
                    result.push_str(value);
                }
                _ => {
                    result.push(ch);
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

pub fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();
    if let Some(c) = chars.next() {
        if !c.is_ascii_alphabetic() && c != '_' {
            return false;
        }
    }

    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return false;
        }
    }

    true
}
