const { toml } = require("./peisar");

console.log(
  toml.parseToml(String.raw`
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

    `),
);
