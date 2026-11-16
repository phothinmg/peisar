use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

use super::date_time::TomlDatetime;

#[derive(Debug, PartialEq, Clone)]
pub enum TomlValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<TomlValue>),
    Table(HashMap<String, TomlValue>),
    Datetime(TomlDatetime),
}

#[derive(Debug, PartialEq, Clone)]
pub enum TomlToken {
    Identifier(String), // e.g., host, port
    String(String),     // e.g., "localhost"
    Integer(i64),       // e.g., 8080
    Float(f64),         // e.g., 3.14
    Boolean(bool),      // true or false
    Equals,             // =
    LBracket,           // [
    RBracket,           // ]
    Comma,              // ,
    Dot,                // .
    Newline,            // \n
    Eof,                // End of file
    Datetime(TomlDatetime),
    RBrace,
    LBrace,
}

pub struct TomlLexer<'a> {
    input: Peekable<Chars<'a>>,
}

impl<'a> TomlLexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            input: source.chars().peekable(),
        }
    }
    pub fn next_token(&mut self) -> Result<TomlToken, String> {
        while let Some(&ch) = self.input.peek() {
            match ch {
                // Skip spaces and tabs
                ' ' | '\t' | '\r' => {
                    self.input.next();
                }
                // Skip comments until newline
                '#' => {
                    while let Some(&c) = self.input.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.input.next();
                    }
                }
                '\n' => {
                    self.input.next();
                    return Ok(TomlToken::Newline);
                }
                '=' => {
                    self.input.next();
                    return Ok(TomlToken::Equals);
                }
                '[' => {
                    self.input.next();
                    return Ok(TomlToken::LBracket);
                }
                ']' => {
                    self.input.next();
                    return Ok(TomlToken::RBracket);
                }
                '{' => {
                    self.input.next();
                    return Ok(TomlToken::LBrace);
                }
                '}' => {
                    self.input.next();
                    return Ok(TomlToken::RBrace);
                }
                ',' => {
                    self.input.next();
                    return Ok(TomlToken::Comma);
                }
                '.' => {
                    self.input.next();
                    return Ok(TomlToken::Dot);
                }
                '"' => return self.read_basic_string(),
                '\'' => return self.read_literal_string(),
                '0'..='9' | '-' | '+' => return self.read_number_or_datetime(),
                'a'..='z' | 'A'..='Z' | '_' => return self.read_bare_key_or_bool(),
                _ => return Err(format!("Unexpected character: '{}'", ch)),
            }
        }
        Ok(TomlToken::Eof)
    }
    fn read_basic_string(&mut self) -> Result<TomlToken, String> {
        if self.starts_with("\"\"\"") {
            // Consume opening `"""`
            self.input.next();
            self.input.next();
            self.input.next();
            return self.read_multiline_basic_string();
        }

        self.input.next(); // consume opening '"'
        let mut s = String::new();
        while let Some(ch) = self.input.next() {
            match ch {
                '"' => return Ok(TomlToken::String(s)),
                '\n' | '\r' => return Err("Unescaped newline in single-line basic string".into()),
                '\\' => s.push(self.read_escape_char()?),
                _ => s.push(ch),
            }
        }
        Err("Unterminated basic string".into())
    }

    fn read_multiline_basic_string(&mut self) -> Result<TomlToken, String> {
        // TOML trims the first newline immediately after opening `"""`
        self.trim_leading_newline();

        let mut s = String::new();
        while self.input.peek().is_some() {
            if self.starts_with("\"\"\"") {
                self.input.next();
                self.input.next();
                self.input.next();

                // TOML allows up to 2 extra quotes right before the closing `"""` (e.g., `""""""`)
                let mut extra_quotes = 0;
                while self.input.peek() == Some(&'"') && extra_quotes < 2 {
                    s.push('"');
                    self.input.next();
                    extra_quotes += 1;
                }
                return Ok(TomlToken::String(s));
            }

            match self.input.next().unwrap() {
                '\\' => {
                    // Line-ending backslash: trims the backslash, newline, and any leading whitespace
                    if matches!(self.input.peek(), Some(&'\n' | &'\r' | &' ' | &'\t')) {
                        let mut lookahead = self.input.clone();
                        while matches!(lookahead.peek(), Some(&' ' | &'\t')) {
                            lookahead.next();
                        }
                        if matches!(lookahead.peek(), Some(&'\n' | &'\r')) {
                            while matches!(self.input.peek(), Some(&' ' | &'\t' | &'\n' | &'\r')) {
                                self.input.next();
                            }
                            continue;
                        }
                    }
                    s.push(self.read_escape_char()?);
                }
                ch => s.push(ch),
            }
        }
        Err("Unterminated multiline basic string".into())
    }

    fn read_escape_char(&mut self) -> Result<char, String> {
        let escaped = self.input.next().ok_or("Unterminated escape sequence")?;
        match escaped {
            'b' => Ok('\u{0008}'),
            't' => Ok('\t'),
            'n' => Ok('\n'),
            'f' => Ok('\u{000C}'),
            'r' => Ok('\r'),
            '"' => Ok('"'),
            '\\' => Ok('\\'),
            'u' => self.read_unicode_escape(4),
            'U' => self.read_unicode_escape(8),
            _ => Err(format!("Invalid escape sequence: \\{}", escaped)),
        }
    }

    fn read_unicode_escape(&mut self, len: usize) -> Result<char, String> {
        let mut hex = String::with_capacity(len);
        for _ in 0..len {
            let ch = self.input.next().ok_or("Incomplete unicode escape")?;
            hex.push(ch);
        }
        let code = u32::from_str_radix(&hex, 16).map_err(|_| format!("Invalid hex: {}", hex))?;
        char::from_u32(code).ok_or_else(|| format!("Invalid unicode scalar: {}", hex))
    }

    // pub fn read_string(&mut self) -> Result<TomlToken, String> {
    //     self.input.next(); // consume opening '"'
    //     let mut s = String::new();
    //     while let Some(ch) = self.input.next() {
    //         match ch {
    //             '"' => return Ok(TomlToken::String(s)),
    //             '\\' => {
    //                 let escaped = self.input.next().ok_or("Unterminated escape sequence")?;
    //                 match escaped {
    //                     'n' => s.push('\n'),
    //                     't' => s.push('\t'),
    //                     '"' => s.push('"'),
    //                     '\\' => s.push('\\'),
    //                     _ => return Err(format!("Invalid escape: \\{}", escaped)),
    //                 }
    //             }
    //             _ => s.push(ch),
    //         }
    //     }
    //     Err("Unterminated string literal".into())
    // }
    fn read_number_or_datetime(&mut self) -> Result<TomlToken, String> {
        // 1. Check for signed special floats (`+inf`, `-inf`, `+nan`, `-nan`)
        if self.starts_with("+inf")
            || self.starts_with("-inf")
            || self.starts_with("+nan")
            || self.starts_with("-nan")
        {
            let sign = self.input.next().unwrap();
            let word: String = (0..3).filter_map(|_| self.input.next()).collect();
            return match (sign, word.as_str()) {
                ('+', "inf") => Ok(TomlToken::Float(f64::INFINITY)),
                ('-', "inf") => Ok(TomlToken::Float(f64::NEG_INFINITY)),
                (_, "nan") => Ok(TomlToken::Float(f64::NAN)),
                _ => unreachable!(),
            };
        }

        // 2. Check for non-decimal integer bases (`0x`, `0o`, `0b`)
        if self.starts_with("0x") || self.starts_with("0o") || self.starts_with("0b") {
            self.input.next(); // consume '0'
            let prefix = self.input.next().unwrap(); // consume 'x', 'o', or 'b'
            return self.read_radix_integer(prefix);
        }

        // 3. Scan standard decimal integer, float (with `.` or `e`/`E`), or datetime
        let mut raw = String::new();
        let mut is_float = false;
        let mut is_datetime = false;

        while let Some(&ch) = self.input.peek() {
            match ch {
                '0'..='9' | '_' => {
                    if ch != '_' {
                        raw.push(ch);
                    }
                    self.input.next();
                }
                '+' | '-' => {
                    // '-' after start is either a date separator (2026-11-17) or exponent sign (1e-5)
                    if !raw.is_empty() && !raw.ends_with(['e', 'E']) {
                        is_datetime = true;
                    }
                    raw.push(ch);
                    self.input.next();
                }
                'e' | 'E' => {
                    is_float = true;
                    raw.push(ch);
                    self.input.next();
                }
                ':' | 'T' | 't' | 'Z' | 'z' => {
                    is_datetime = true;
                    raw.push(ch);
                    self.input.next();
                }
                ' ' if is_datetime && raw.len() == 10 => {
                    raw.push('T');
                    self.input.next();
                }
                '.' => {
                    if !is_datetime {
                        is_float = true;
                    }
                    raw.push(ch);
                    self.input.next();
                }
                _ => break,
            }
        }

        if is_datetime {
            let dt = TomlDatetime::parse(&raw)?;
            Ok(TomlToken::Datetime(dt))
        } else if is_float {
            raw.parse::<f64>()
                .map(TomlToken::Float)
                .map_err(|_| format!("Invalid float: {}", raw))
        } else {
            // TOML forbids leading zeros on base-10 integers (e.g., `012` is invalid)
            let unsigned = raw.trim_start_matches(['+', '-']);
            if unsigned.len() > 1 && unsigned.starts_with('0') {
                return Err(format!("Leading zeros are not allowed in integer: {}", raw));
            }
            raw.parse::<i64>()
                .map(TomlToken::Integer)
                .map_err(|_| format!("Invalid integer: {}", raw))
        }
    }

    #[allow(unused)]
    fn validate_datetime(s: &str) -> Result<(), String> {
        // Basic structural check for YYYY-MM-DD or HH:MM:SS
        let has_valid_date = s.len() >= 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-';
        let has_valid_time = s.len() >= 8 && s.as_bytes()[2] == b':' && s.as_bytes()[5] == b':';

        if has_valid_date || has_valid_time {
            Ok(())
        } else {
            Err(format!("Invalid TOML datetime format: '{}'", s))
        }
    }

    fn read_bare_key_or_bool(&mut self) -> Result<TomlToken, String> {
        let mut ident = String::new();
        while let Some(&ch) = self.input.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ident.push(ch);
                self.input.next();
            } else {
                break;
            }
        }

        match ident.as_str() {
            "true" => Ok(TomlToken::Boolean(true)),
            "false" => Ok(TomlToken::Boolean(false)),
            "inf" => Ok(TomlToken::Float(f64::INFINITY)),
            "nan" => Ok(TomlToken::Float(f64::NAN)),
            _ => Ok(TomlToken::Identifier(ident)),
        }
    }

    fn starts_with(&self, prefix: &str) -> bool {
        // Clone the underlying Peekable iterator (cheap pointer copy on str::Chars)
        let mut iter = self.input.clone();
        for expected in prefix.chars() {
            if iter.next() != Some(expected) {
                return false;
            }
        }
        true
    }

    fn trim_leading_newline(&mut self) {
        if self.input.peek() == Some(&'\r') {
            self.input.next();
        }
        if self.input.peek() == Some(&'\n') {
            self.input.next();
        }
    }
    fn read_literal_string(&mut self) -> Result<TomlToken, String> {
        if self.starts_with("'''") {
            // Consume opening `'''`
            self.input.next();
            self.input.next();
            self.input.next();
            return self.read_multiline_literal_string();
        }

        self.input.next(); // consume opening '\''
        let mut s = String::new();
        while let Some(ch) = self.input.next() {
            match ch {
                '\'' => return Ok(TomlToken::String(s)),
                '\n' | '\r' => {
                    return Err("Newline not allowed in single-line literal string".into());
                }
                _ => s.push(ch),
            }
        }
        Err("Unterminated literal string".into())
    }

    fn read_multiline_literal_string(&mut self) -> Result<TomlToken, String> {
        // TOML trims the first newline immediately after opening `'''`
        self.trim_leading_newline();

        let mut s = String::new();
        while self.input.peek().is_some() {
            if self.starts_with("'''") {
                self.input.next();
                self.input.next();
                self.input.next();

                // Allow up to 2 extra single quotes before closing `'''`
                let mut extra_quotes = 0;
                while self.input.peek() == Some(&'\'') && extra_quotes < 2 {
                    s.push('\'');
                    self.input.next();
                    extra_quotes += 1;
                }
                return Ok(TomlToken::String(s));
            }

            s.push(self.input.next().unwrap());
        }
        Err("Unterminated multiline literal string".into())
    }
    fn read_radix_integer(&mut self, prefix: char) -> Result<TomlToken, String> {
        let radix = match prefix {
            'x' => 16,
            'o' => 8,
            'b' => 2,
            _ => unreachable!(),
        };

        let mut digits = String::new();
        while let Some(&ch) = self.input.peek() {
            if ch == '_' {
                self.input.next();
            } else if ch.is_digit(radix) {
                digits.push(ch);
                self.input.next();
            } else if ch.is_ascii_alphanumeric() {
                return Err(format!("Invalid digit '{}' for base-{} integer", ch, radix));
            } else {
                break;
            }
        }

        if digits.is_empty() {
            return Err(format!("Expected digits after 0{}", prefix));
        }

        i64::from_str_radix(&digits, radix)
            .map(TomlToken::Integer)
            .map_err(|_| format!("Invalid base-{} integer: 0{}{}", radix, prefix, digits))
    }
}
