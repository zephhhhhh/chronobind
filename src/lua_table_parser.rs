use std::collections::HashMap;

/// Represents a Lua value.
#[derive(Debug, Clone, PartialEq)]
pub enum LuaValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Table(HashMap<String, Self>),
}

impl LuaValue {
    /// Returns the value as a string slice, if it is a string.
    #[must_use]
    pub fn as_string(&self) -> Option<&str> {
        if let Self::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// Returns the value as a f64, if it is a number.
    #[must_use]
    pub const fn number_as_f64(&self) -> Option<f64> {
        if let Self::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// Returns the value truncated to a i64, if it is a number.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn number_as_i64(&self) -> Option<i64> {
        if let Self::Number(n) = self {
            Some(n.trunc() as i64)
        } else {
            None
        }
    }

    /// Returns the value as a table, if it is a table.
    #[must_use]
    pub const fn as_table(&self) -> Option<&HashMap<String, Self>> {
        if let Self::Table(t) = self {
            Some(t)
        } else {
            None
        }
    }
}

/// A simple Lua table parser.
#[derive(Debug, Clone)]
#[must_use]
pub struct LuaTableParser<'a> {
    chars: std::str::Chars<'a>,
    lookahead: Option<char>,
}

impl<'a> LuaTableParser<'a> {
    /// Creates a new `LuaTableParser`.
    #[inline]
    pub fn new(input: &'a str) -> Self {
        let mut chars = input.chars();
        let lookahead = chars.next();
        LuaTableParser { chars, lookahead }
    }

    /// Peeks at the next character without consuming it.
    #[inline]
    #[must_use]
    const fn peek(&self) -> Option<char> {
        self.lookahead
    }

    /// Advance to the next character.
    #[inline]
    fn bump(&mut self) {
        self.lookahead = self.chars.next();
    }

    /// Keep advancing the parser until a non-whitespace character is found.
    #[inline]
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek()
            && c.is_whitespace()
        {
            self.bump();
        }
    }

    /// Expect the next character to be `c`, consuming it if so.
    #[inline]
    fn expect(&mut self, c: char) -> bool {
        self.skip_whitespace();
        let Some(p) = self.peek() else {
            return false;
        };
        if p == c {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Parse an identifier from the current position.
    #[inline]
    fn parse_identifier(&mut self) -> Option<String> {
        self.skip_whitespace();
        let mut s = String::new();

        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
            s.push(self.peek()?);
            self.bump();
        }

        if s.is_empty() {
            return None;
        }

        Some(s)
    }

    /// Parse a Lua string from the current position.
    #[inline]
    fn parse_string(&mut self) -> Option<String> {
        if !self.expect('"') {
            return None;
        }
        let mut out = String::new();
        while let Some(c) = self.peek()
            && c != '"'
        {
            out.push(c);
            self.bump();
        }
        if !self.expect('"') {
            return None;
        }
        Some(out)
    }

    /// Parse a Lua number from the current position.
    #[inline]
    fn parse_number(&mut self) -> Option<f64> {
        self.skip_whitespace();
        let mut s = String::new();
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.' || c == '-' || c == '+')
        {
            s.push(self.peek()?);
            self.bump();
        }
        s.parse().ok()
    }

    /// Parse a Lua value from the current position.
    #[inline]
    fn parse_value(&mut self) -> Option<LuaValue> {
        self.skip_whitespace();
        match self.peek() {
            Some('"') => Some(LuaValue::String(self.parse_string()?)),
            Some('{') => Some(LuaValue::Table(self.parse_table()?)),
            Some(c) if c.is_ascii_digit() => Some(LuaValue::Number(self.parse_number()?)),
            _ => {
                log::warn!(
                    "Unexpected character while parsing value: {:?}",
                    self.peek()
                );
                None
            }
        }
    }

    /// Parse a key into a Lua table from the current position.
    #[inline]
    fn parse_key(&mut self) -> Option<String> {
        self.skip_whitespace();
        self.expect('[');
        let key = self.parse_string()?;
        self.expect(']');
        Some(key)
    }

    /// Parse a Lua table from the current position.
    #[inline]
    fn parse_table(&mut self) -> Option<HashMap<String, LuaValue>> {
        pub const SAFETY_LIMIT: usize = 10_000;

        let mut map = HashMap::new();
        self.expect('{');

        for _ in 0..SAFETY_LIMIT {
            self.skip_whitespace();
            if self.peek() == Some('}') {
                self.bump();
                break;
            }

            let key = self.parse_key()?;
            self.skip_whitespace();
            self.expect('=');
            let value = self.parse_value()?;
            map.insert(key, value);

            self.skip_whitespace();
            if self.peek() == Some(',') {
                self.bump();
            }
        }

        Some(map)
    }

    /// Parse global variables from the Lua script.
    #[inline]
    pub fn parse_globals(&mut self) -> Option<HashMap<String, LuaValue>> {
        let mut globals = HashMap::new();

        while self.peek().is_some() {
            self.skip_whitespace();
            if self.peek().is_none() {
                break;
            }

            let name = self.parse_identifier()?;
            self.skip_whitespace();
            self.expect('=');
            let value = self.parse_value()?;

            globals.insert(name, value);

            self.skip_whitespace();
        }

        Some(globals)
    }
}
