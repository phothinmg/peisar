//! A minimal TOML parser written in Rust without external libraries.
//!
//! The NAPI binding exposes [`parseToml`](fn@parse_toml) as a JavaScript
//! function that returns the parsed document as a native object.

mod lexer;
use std::collections::HashMap;
mod date_time;

use lexer::{TomlLexer, TomlToken, TomlValue};

pub struct TomlParser<'a> {
    lexer: TomlLexer<'a>,
    current: TomlToken,
    current_path: Vec<String>,
}

impl<'a> TomlParser<'a> {
    pub fn new(source: &'a str) -> Result<Self, String> {
        let mut lexer = TomlLexer::new(source);
        let current = lexer.next_token()?;
        Ok(Self {
            lexer,
            current,
            current_path: Vec::new(),
        })
    }

    pub fn advance(&mut self) -> Result<TomlToken, String> {
        let prev = std::mem::replace(&mut self.current, self.lexer.next_token()?);
        Ok(prev)
    }
    pub fn expect(&mut self, expected: TomlToken) -> Result<(), String> {
        if self.current == expected {
            self.advance()?;
            Ok(())
        } else {
            Err(format!("Expected {:?}, found {:?}", expected, self.current))
        }
    }
    pub fn parse(&mut self) -> Result<HashMap<String, TomlValue>, String> {
        let mut root = HashMap::new();

        while self.current != TomlToken::Eof {
            match &self.current {
                TomlToken::Newline => {
                    self.advance()?;
                }
                TomlToken::LBracket => {
                    self.parse_table_header(&mut root)?;
                }
                TomlToken::Identifier(_) | TomlToken::String(_) => {
                    // Resolve the active table (using self.current_path) before parsing the key-value
                    let active_table = Self::get_or_create_table(&mut root, &self.current_path)?;
                    self.parse_key_value(active_table)?;
                }
                _ => return Err(format!("Unexpected token at top level: {:?}", self.current)),
            }
        }

        Ok(root)
    }

    pub fn parse_table_header(
        &mut self,
        root: &mut HashMap<String, TomlValue>,
    ) -> Result<(), String> {
        self.expect(TomlToken::LBracket)?;

        // Check for the second '[' in `[[items]]`
        let is_array_of_tables = if self.current == TomlToken::LBracket {
            self.advance()?;
            true
        } else {
            false
        };

        let path = self.parse_key_path()?;

        self.expect(TomlToken::RBracket)?;
        if is_array_of_tables {
            self.expect(TomlToken::RBracket)?; // Consume the second ']'
            Self::push_table_to_array(root, &path)?;
        } else {
            Self::get_or_create_table(root, &path)?;
        }

        self.current_path = path;
        Ok(())
    }

    pub fn parse_key_value(
        &mut self,
        target_table: &mut HashMap<String, TomlValue>,
    ) -> Result<(), String> {
        // 1. Read the full dotted path (e.g., ["type", "name"]) instead of a single key
        let key_path = self.parse_key_path()?;

        // 2. Now expect '='
        self.expect(TomlToken::Equals)?;
        let value = self.parse_value()?;

        // 3. Split into parent path (["type"]) and final key ("name")
        let (final_key, parent_path) = key_path.split_last().ok_or("Key path cannot be empty")?;

        // 4. Create intermediate tables and insert the final key
        let destination = Self::get_or_create_table(target_table, parent_path)?;
        if destination.insert(final_key.clone(), value).is_some() {
            return Err(format!("Duplicate key: '{}'", final_key));
        }
        Ok(())
    }
    pub fn parse_key_path(&mut self) -> Result<Vec<String>, String> {
        let mut path = Vec::new();
        loop {
            match self.advance()? {
                TomlToken::Identifier(name) | TomlToken::String(name) => path.push(name),
                t => return Err(format!("Expected key name, got {:?}", t)),
            }

            if self.current == TomlToken::Dot {
                self.advance()?; // Consume '.' and continue loop
            } else {
                break;
            }
        }
        Ok(path)
    }

    pub fn parse_value(&mut self) -> Result<TomlValue, String> {
        match self.advance()? {
            TomlToken::String(s) => Ok(TomlValue::String(s)),
            TomlToken::Integer(i) => Ok(TomlValue::Integer(i)),
            TomlToken::Float(f) => Ok(TomlValue::Float(f)),
            TomlToken::Boolean(b) => Ok(TomlValue::Boolean(b)),
            TomlToken::Datetime(dt) => Ok(TomlValue::Datetime(dt)), // Map token to AST
            TomlToken::LBracket => self.parse_array(),
            TomlToken::LBrace => self.parse_inline_table(),
            t => Err(format!("Unexpected value token: {:?}", t)),
        }
    }
    pub fn parse_array(&mut self) -> Result<TomlValue, String> {
        let mut items = Vec::new();

        while self.current != TomlToken::RBracket {
            if self.current == TomlToken::Newline {
                self.advance()?;
                continue;
            }
            items.push(self.parse_value()?);

            if self.current == TomlToken::Comma {
                self.advance()?;
            } else {
                break;
            }
        }

        // Skip trailing newlines before closing bracket
        while self.current == TomlToken::Newline {
            self.advance()?;
        }

        self.expect(TomlToken::RBracket)?;
        Ok(TomlValue::Array(items))
    }
    pub fn push_table_to_array(
        root: &mut HashMap<String, TomlValue>,
        path: &[String],
    ) -> Result<(), String> {
        let (last_key, parent_path) = path
            .split_last()
            .ok_or("Array of tables header cannot be empty")?;

        // Navigate to the parent table
        let parent = Self::get_or_create_table(root, parent_path)?;

        // Ensure the target key is a TomlValue::Array
        let entry = parent
            .entry(last_key.clone())
            .or_insert_with(|| TomlValue::Array(Vec::new()));

        match entry {
            TomlValue::Array(vec) => {
                vec.push(TomlValue::Table(HashMap::new()));
                Ok(())
            }
            _ => Err(format!(
                "'{}' is already defined and is not an array of tables",
                last_key
            )),
        }
    }
    pub fn get_or_create_table<'b>(
        root: &'b mut HashMap<String, TomlValue>,
        path: &[String],
    ) -> Result<&'b mut HashMap<String, TomlValue>, String> {
        let mut current = root;

        for segment in path {
            let entry = current
                .entry(segment.clone())
                .or_insert_with(|| TomlValue::Table(HashMap::new()));

            current = match entry {
                TomlValue::Table(map) => map,
                // Essential: enter the most recently pushed table in the array
                TomlValue::Array(vec) => match vec.last_mut() {
                    Some(TomlValue::Table(map)) => map,
                    _ => return Err(format!("Array '{}' does not contain tables", segment)),
                },
                _ => {
                    return Err(format!(
                        "'{}' is already defined as a non-table value",
                        segment
                    ));
                }
            };
        }

        Ok(current)
    }
    pub fn parse_inline_table(&mut self) -> Result<TomlValue, String> {
        let mut table = HashMap::new();

        // Handle empty inline table `{}`
        if self.current == TomlToken::RBrace {
            self.advance()?;
            return Ok(TomlValue::Table(table));
        }

        loop {
            // TOML v1.0 forbids newlines inside inline tables
            if self.current == TomlToken::Newline {
                return Err("Newlines are not allowed inside inline tables".into());
            }

            self.parse_key_value(&mut table)?;

            if self.current == TomlToken::Comma {
                self.advance()?;
                // TOML v1.0 forbids trailing commas in inline tables
                if self.current == TomlToken::RBrace {
                    return Err("Trailing comma is not allowed in inline tables".into());
                }
            } else {
                break;
            }
        }

        self.expect(TomlToken::RBrace)?;
        Ok(TomlValue::Table(table))
    }
}

#[cfg(feature = "npm")]
#[napi_derive::napi(
    js_name = "parseToml",
    ts_return_type = "Record<string,unknown>",
    namespace = "toml"
)]
/// Parses TOML source and returns its contents as a JavaScript object.
///
/// TOML datetime values are returned as ISO-8601 strings.
pub fn parse_toml<'env>(
    env: &'env napi::Env,
    source: String,
) -> napi::Result<napi::bindgen_prelude::Object<'env>> {
    let parsed = parse_toml_source(&source).map_err(napi::Error::from_reason)?;
    values_to_js_object(&env, parsed)
}

#[cfg(feature = "npm")]
#[napi_derive::napi(
    js_name = "parseTomlAsync",
    ts_return_type = "Promise<Record<string, unknown>>",
    namespace = "toml"
)]
/// Parses TOML source asynchronously and returns its contents as a JavaScript object.
///
/// TOML datetime values are returned as ISO-8601 strings.
pub fn parse_toml_async(source: String) -> napi::bindgen_prelude::AsyncTask<TomlParseTask> {
    napi::bindgen_prelude::AsyncTask::new(TomlParseTask { source })
}

#[cfg(feature = "npm")]
pub struct TomlParseTask {
    source: String,
}

#[cfg(feature = "npm")]
impl<'task> napi::ScopedTask<'task> for TomlParseTask {
    type Output = HashMap<String, TomlValue>;
    type JsValue = napi::bindgen_prelude::Object<'task>;

    fn compute(&mut self) -> napi::Result<Self::Output> {
        parse_toml_source(&self.source).map_err(napi::Error::from_reason)
    }

    fn resolve(
        &mut self,
        env: &'task napi::Env,
        output: Self::Output,
    ) -> napi::Result<Self::JsValue> {
        values_to_js_object(env, output)
    }
}

#[cfg(feature = "npm")]
fn parse_toml_source(source: &str) -> Result<HashMap<String, TomlValue>, String> {
    let mut parser = TomlParser::new(source)?;
    parser.parse()
}

#[cfg(feature = "npm")]
fn values_to_js_object<'env>(
    env: &'env napi::Env,
    values: HashMap<String, TomlValue>,
) -> napi::Result<napi::bindgen_prelude::Object<'env>> {
    let mut object = napi::bindgen_prelude::Object::new(env)?;
    for (key, value) in values {
        set_js_object_property(env, &mut object, &key, value)?;
    }
    Ok(object)
}

#[cfg(feature = "npm")]
fn set_js_object_property(
    env: &napi::Env,
    object: &mut napi::bindgen_prelude::Object,
    key: &str,
    value: TomlValue,
) -> napi::Result<()> {
    use napi::bindgen_prelude::JsObjectValue;

    match value {
        TomlValue::String(value) => object.set_named_property(key, value),
        TomlValue::Integer(value) => object.set_named_property(key, value),
        TomlValue::Float(value) => object.set_named_property(key, value),
        TomlValue::Boolean(value) => object.set_named_property(key, value),
        TomlValue::Array(values) => {
            object.set_named_property(key, values_to_js_array(env, values)?)
        }
        TomlValue::Table(values) => {
            object.set_named_property(key, values_to_js_object(env, values)?)
        }
        TomlValue::Datetime(value) => object.set_named_property(key, datetime_to_string(value)),
    }
}

#[cfg(feature = "npm")]
fn values_to_js_array<'env>(
    env: &'env napi::Env,
    values: Vec<TomlValue>,
) -> napi::Result<napi::bindgen_prelude::Array<'env>> {
    let length = u32::try_from(values.len())
        .map_err(|_| napi::Error::from_reason("TOML array exceeds JavaScript array limit"))?;
    let mut array = env.create_array(length)?;
    for (index, value) in values.into_iter().enumerate() {
        set_js_array_element(env, &mut array, index as u32, value)?;
    }
    Ok(array)
}

#[cfg(feature = "npm")]
fn set_js_array_element(
    env: &napi::Env,
    array: &mut napi::bindgen_prelude::Array,
    index: u32,
    value: TomlValue,
) -> napi::Result<()> {
    match value {
        TomlValue::String(value) => array.set(index, value),
        TomlValue::Integer(value) => array.set(index, value),
        TomlValue::Float(value) => array.set(index, value),
        TomlValue::Boolean(value) => array.set(index, value),
        TomlValue::Array(values) => array.set(index, values_to_js_array(env, values)?),
        TomlValue::Table(values) => array.set(index, values_to_js_object(env, values)?),
        TomlValue::Datetime(value) => array.set(index, datetime_to_string(value)),
    }
}

#[cfg(feature = "npm")]
fn datetime_to_string(value: date_time::TomlDatetime) -> String {
    use std::fmt::Write;

    let mut result = String::new();
    if let Some(date) = value.date {
        write!(
            &mut result,
            "{:04}-{:02}-{:02}",
            date.year, date.month, date.day
        )
        .unwrap();
    }
    if let Some(time) = value.time {
        if !result.is_empty() {
            result.push('T');
        }
        write!(
            &mut result,
            "{:02}:{:02}:{:02}",
            time.hour, time.minute, time.second
        )
        .unwrap();
        if time.nanosecond != 0 {
            let fraction = format!("{:09}", time.nanosecond);
            result.push('.');
            result.push_str(fraction.trim_end_matches('0'));
        }
    }
    if let Some(offset) = value.offset {
        match offset {
            date_time::TomlTzOffset::Z => result.push('Z'),
            date_time::TomlTzOffset::Custom { minutes } => {
                let sign = if minutes < 0 { '-' } else { '+' };
                let absolute_minutes = minutes.abs();
                write!(
                    &mut result,
                    "{sign}{:02}:{:02}",
                    absolute_minutes / 60,
                    absolute_minutes % 60
                )
                .unwrap();
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_toml(input: &str) -> Result<HashMap<String, TomlValue>, String> {
        let mut parser = TomlParser::new(input)?;
        parser.parse()
    }

    #[test]
    fn test_dotted_keys_and_inline_tables() {
        let input = r#"
            animal = { type.name = "pug", stats = { age = 4 } }
        "#;
        let ast = parse_toml(input).expect("Should parse valid TOML");
        assert!(ast.contains_key("animal"));
    }

    #[test]
    fn test_arrays_of_tables() {
        let input = r#"
            [[items]]
            name = "Sword"
            [[items]]
            name = "Shield"
        "#;
        let ast = parse_toml(input).expect("Should parse array of tables");
        if let Some(TomlValue::Array(items)) = ast.get("items") {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected 'items' to be an Array");
        }
    }

    #[test]
    fn test_invalid_syntax_errors() {
        assert!(parse_toml("duplicate = 1\nduplicate = 2").is_err());
        assert!(parse_toml("bad_inline = { a = 1,\n b = 2 }").is_err());
    }

    #[cfg(feature = "npm")]
    #[test]
    fn napi_source_parser_accepts_extended_toml_values() {
        let parsed = super::parse_toml_source(
            "message = \"\"\"\nfirst line\nsecond line\"\"\"\nhex = 0xFF\npositive_inf = inf\nnot_a_number = nan",
        )
        .expect("extended TOML values should parse");

        assert_eq!(
            parsed.get("message"),
            Some(&TomlValue::String("first line\nsecond line".to_string()))
        );
        assert_eq!(parsed.get("hex"), Some(&TomlValue::Integer(255)));
        assert!(matches!(
            parsed.get("positive_inf"),
            Some(TomlValue::Float(value)) if value.is_infinite() && value.is_sign_positive()
        ));
        assert!(matches!(
            parsed.get("not_a_number"),
            Some(TomlValue::Float(value)) if value.is_nan()
        ));
    }
}
