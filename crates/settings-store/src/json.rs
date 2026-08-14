use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

pub(crate) fn parse(input: &str) -> Result<Value, String> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        offset: 0,
    };
    let value = parser.value()?;
    parser.whitespace();
    if parser.offset != parser.bytes.len() {
        return Err(format!("trailing content at byte {}", parser.offset));
    }
    Ok(value)
}

pub(crate) fn stringify(value: &Value) -> String {
    let mut output = String::new();
    write_value(value, &mut output);
    output
}

fn write_value(value: &Value, output: &mut String) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => write!(output, "{value}").unwrap(),
        Value::String(value) => write_string(value, output),
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_value(value, output);
            }
            output.push(']');
        }
        Value::Object(values) => {
            output.push('{');
            for (index, (key, value)) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_string(key, output);
                output.push(':');
                write_value(value, output);
            }
            output.push('}');
        }
    }
}

fn write_string(value: &str, output: &mut String) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character < ' ' => write!(output, "\\u{:04x}", character as u32).unwrap(),
            character => output.push(character),
        }
    }
    output.push('"');
}

struct Parser<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Parser<'_> {
    fn value(&mut self) -> Result<Value, String> {
        self.whitespace();
        match self.peek() {
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(Value::Null)
            }
            Some(b't') => {
                self.literal(b"true")?;
                Ok(Value::Bool(true))
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(Value::Bool(false))
            }
            Some(b'"') => self.string().map(Value::String),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'-' | b'0'..=b'9') => self.number().map(Value::Number),
            _ => Err(format!("expected JSON value at byte {}", self.offset)),
        }
    }

    fn object(&mut self) -> Result<Value, String> {
        self.expect(b'{')?;
        let mut values = BTreeMap::new();
        self.whitespace();
        if self.consume(b'}') {
            return Ok(Value::Object(values));
        }
        loop {
            self.whitespace();
            let key = self.string()?;
            self.whitespace();
            self.expect(b':')?;
            if values.insert(key, self.value()?).is_some() {
                return Err("duplicate object key".into());
            }
            self.whitespace();
            if self.consume(b'}') {
                break;
            }
            self.expect(b',')?;
        }
        Ok(Value::Object(values))
    }

    fn array(&mut self) -> Result<Value, String> {
        self.expect(b'[')?;
        let mut values = Vec::new();
        self.whitespace();
        if self.consume(b']') {
            return Ok(Value::Array(values));
        }
        loop {
            values.push(self.value()?);
            self.whitespace();
            if self.consume(b']') {
                break;
            }
            self.expect(b',')?;
        }
        Ok(Value::Array(values))
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut output = String::new();
        while let Some(byte) = self.next() {
            match byte {
                b'"' => return Ok(output),
                b'\\' => match self.next() {
                    Some(b'"') => output.push('"'),
                    Some(b'\\') => output.push('\\'),
                    Some(b'/') => output.push('/'),
                    Some(b'b') => output.push('\u{0008}'),
                    Some(b'f') => output.push('\u{000c}'),
                    Some(b'n') => output.push('\n'),
                    Some(b'r') => output.push('\r'),
                    Some(b't') => output.push('\t'),
                    Some(b'u') => {
                        let mut code = 0_u32;
                        for _ in 0..4 {
                            let digit = self
                                .next()
                                .and_then(|value| (value as char).to_digit(16))
                                .ok_or_else(|| "invalid unicode escape".to_string())?;
                            code = code * 16 + digit;
                        }
                        output.push(
                            char::from_u32(code)
                                .ok_or_else(|| "invalid unicode scalar".to_string())?,
                        );
                    }
                    _ => return Err("invalid string escape".into()),
                },
                0..=31 => return Err("control character in string".into()),
                _ => {
                    self.offset -= 1;
                    let remainder = std::str::from_utf8(&self.bytes[self.offset..])
                        .map_err(|_| "invalid UTF-8")?;
                    let character = remainder.chars().next().ok_or("truncated UTF-8")?;
                    output.push(character);
                    self.offset += character.len_utf8();
                }
            }
        }
        Err("unterminated string".into())
    }

    fn number(&mut self) -> Result<i64, String> {
        let start = self.offset;
        self.consume(b'-');
        if self.consume(b'0') {
            if matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err("leading zero".into());
            }
        } else {
            let digits = self.offset;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
            if self.offset == digits {
                return Err("expected integer".into());
            }
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err("non-integer number unsupported".into());
        }
        std::str::from_utf8(&self.bytes[start..self.offset])
            .unwrap()
            .parse()
            .map_err(|_| "integer out of range".into())
    }

    fn literal(&mut self, literal: &[u8]) -> Result<(), String> {
        if self.bytes.get(self.offset..self.offset + literal.len()) == Some(literal) {
            self.offset += literal.len();
            Ok(())
        } else {
            Err(format!("invalid literal at byte {}", self.offset))
        }
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.offset += 1;
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), String> {
        if self.consume(expected) {
            Ok(())
        } else {
            Err(format!(
                "expected '{}' at byte {}",
                expected as char, self.offset
            ))
        }
    }

    fn consume(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn next(&mut self) -> Option<u8> {
        let value = self.peek()?;
        self.offset += 1;
        Some(value)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.offset).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_nested_json_deterministically() {
        let input = r#"{"a":[true,null,"繁中",-12],"z":{"escaped":"a\\nb"}}"#;
        let value = parse(input).unwrap();
        assert_eq!(parse(&stringify(&value)).unwrap(), value);
        assert_eq!(stringify(&value), input);
    }

    #[test]
    fn rejects_duplicate_keys_and_partial_documents() {
        assert!(parse(r#"{"a":1,"a":2}"#).is_err());
        assert!(parse(r#"{"a":[1,2}"#).is_err());
    }
}
