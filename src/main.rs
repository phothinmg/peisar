use peisar::toml::TomlParser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sample_toml = r#"
    winpath  = 'C:\Users\nodejs\templates'
    regex    = '<\i\c*\s*>'

    multiline_basic = """
    Roses are red
    Violets are blue"""

    line_continuation = """
    The quick brown \
    fox jumps over \
    the lazy dog."""

    multiline_literal = '''
    The first newline is
    trimmed in raw strings.
    All other whitespace
    is preserved.
    '''
    "#;

    let mut parser = TomlParser::new(sample_toml)?;
    let ast = parser.parse()?;

    println!("{:#?}", ast);
    Ok(())
}
