/*
 * Copyright (c) 2022, Andreas Kling <kling@serenityos.org>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug)]
pub enum Value {
    Null,
    Number(Number),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

#[derive(Debug)]
pub enum Number {
    Integer64(i64),
    Float64(f64),
}

#[derive(Debug)]
pub enum ParseError {
    NotImplemented,
    ExpectedCharacter(char),
    ExpectedFalse,
    ExpectedNull,
    ExpectedTrue,
    InvalidNumber,
    MultiplePeriodsInNumber,
    UnexpectedCharacter(char),
    UnexpectedControlCharacter,
    UnexpectedEof,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> { todo!() }
}

impl std::error::Error for ParseError {}

struct JsonParser<'a> {
    input: &'a mut Peekable<Chars<'a>>,
}

impl<'a> JsonParser<'a> {
    fn parse(&mut self) -> Result<Value, ParseError> {
        self.skip_whitespace();
        match self.input.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string(),
            Some('0'..='9' | '-') => self.parse_number(),
            Some('t') => self.parse_true(),
            Some('f') => self.parse_false(),
            Some('n') => self.parse_null(),
            Some(ch) => Err(ParseError::UnexpectedCharacter(*ch)),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn must_consume(&mut self, ch: char) -> Result<(), ParseError> {
        match self.input.peek() {
            Some(peek_ch) if ch == *peek_ch => {
                self.input.next();
                Ok(())
            }
            _ => Err(ParseError::ExpectedCharacter(ch)),
        }
    }

    fn parse_object(&mut self) -> Result<Value, ParseError> {
        self.must_consume('{')?;

        let mut object = HashMap::new();

        loop {
            self.skip_whitespace();
            let name = self.consume_and_unescape_string()?;
            self.skip_whitespace();
            self.must_consume(':')?;
            self.skip_whitespace();
            let value = self.parse()?;
            object.insert(name, value);
            self.skip_whitespace();
            if let Some('}') = self.input.peek() {
                break;
            }
            self.must_consume(',')?;
        }
        self.skip_whitespace();
        self.must_consume('}')?;
        Ok(Value::Object(object))
    }

    fn parse_array(&mut self) -> Result<Value, ParseError> {
        self.must_consume('[')?;
        let mut array = Vec::new();

        loop {
            self.skip_whitespace();
            if self.next_is("]") {
                break;
            }
            let element = self.parse()?;
            array.push(element);
            self.skip_whitespace();
            if self.next_is("]") {
                break;
            }
            self.must_consume(',')?;
            self.skip_whitespace();
            if self.next_is("]") {
                return Err(ParseError::UnexpectedCharacter(']'));
            }
        }
        self.skip_whitespace();
        self.must_consume(']')?;
        Ok(Value::Array(array))
    }

    fn parse_number(&mut self) -> Result<Value, ParseError> {
        let mut string = String::with_capacity(64);

        let mut has_decimals = false;
        let mut all_zero = true;

        loop {
            let ch = self.input.peek();
            match ch {
                Some('.') => {
                    if has_decimals {
                        return Err(ParseError::MultiplePeriodsInNumber);
                    }
                    has_decimals = true;
                }
                ch @ Some('-' | '0'..='9') => {
                    let ch = *ch.unwrap();
                    if ch != '-' && ch != '0' {
                        all_zero = false;
                    }
                    if has_decimals {
                        if ch == '-' {
                            return Err(ParseError::InvalidNumber);
                        }
                    } else {
                        if string.starts_with("0") || string.starts_with("-0") {
                            return Err(ParseError::InvalidNumber);
                        }
                    }
                }
                _ => break,
            }
            string.push(*ch.unwrap());
            self.input.next();
        }

        if string.starts_with("-") && all_zero {
            return Ok(Value::Number(Number::Float64(-0.0)));
        }

        if has_decimals {
            let result = string.parse::<f64>();
            if result.is_err() {
                Err(ParseError::InvalidNumber)
            } else {
                Ok(Value::Number(Number::Float64(result.unwrap())))
            }
        } else {
            let result = string.parse::<i64>();
            if result.is_err() {
                Err(ParseError::InvalidNumber)
            } else {
                Ok(Value::Number(Number::Integer64(result.unwrap())))
            }
        }
    }

    fn parse_string(&mut self) -> Result<Value, ParseError> {
        let string = self.consume_and_unescape_string()?;
        Ok(Value::String(string))
    }

    fn parse_true(&mut self) -> Result<Value, ParseError> {
        if self.consume("true") {
            Ok(Value::Bool(true))
        } else {
            Err(ParseError::ExpectedTrue)
        }
    }

    fn parse_false(&mut self) -> Result<Value, ParseError> {
        if self.consume("false") {
            Ok(Value::Bool(false))
        } else {
            Err(ParseError::ExpectedFalse)
        }
    }

    fn parse_null(&mut self) -> Result<Value, ParseError> {
        if self.consume("null") {
            Ok(Value::Null)
        } else {
            Err(ParseError::ExpectedNull)
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.input.peek() {
                Some(' ' | '\n' | '\r' | '\t') => {
                    self.input.next();
                }
                _ => break,
            }
        }
    }

    fn consume(&mut self, expected: &str) -> bool {
        if !self.next_is(expected) {
            return false;
        }
        for _ in 0..expected.len() {
            self.input.next();
        }
        return true;
    }

    fn next_is(&self, expected: &str) -> bool {
        let mut lookahead_chars = self.input.clone();
        let mut expected_chars = expected.chars();
        loop {
            let lookahead_character = lookahead_chars.next();
            let expected_character = expected_chars.next();
            if expected_character.is_none() {
                return true;
            }
            if lookahead_character.is_none() {
                return false;
            }
            if lookahead_character != expected_character {
                return false;
            }
        }
    }

    fn consume_and_unescape_string(&mut self) -> Result<String, ParseError> {
        self.must_consume('"')?;

        let mut string = String::new();
        let mut skip_count = 0;
        loop {
            let mut lookahead = self.input.clone();
            let mut ch = '\0';
            loop {
                let peek_ch = lookahead.next();
                if peek_ch.is_none() {
                    break;
                }
                ch = peek_ch.unwrap();
                if ch == '"' || ch == '\\' {
                    break;
                }
                if ch < 0x20 as char {
                    return Err(ParseError::UnexpectedControlCharacter);
                }
                skip_count += 1;
            }

            for _ in 0..skip_count {
                string.push(self.input.next().unwrap());
            }

            if self.input.peek().is_none() {
                break;
            }

            match ch {
                '"' => break,
                '\\' => {
                    // Skip the \
                    self.input.next();

                    match self.input.next() {
                        Some('\\') => string.push('\\'),
                        Some('/') => string.push('/'),
                        Some('n') => string.push('\n'),
                        Some('r') => string.push('\r'),
                        Some('b') => string.push(0x8 as char),
                        Some('f') => string.push(0xc as char),
                        Some(ch) => return Err(ParseError::UnexpectedCharacter(ch)),
                        None => return Err(ParseError::UnexpectedEof),
                    }
                }
                _ => string.push(self.input.next().unwrap()),
            }
        }
        self.must_consume('"')?;
        Ok(string)
    }
}

pub fn parse(input: &str) -> Result<Value, ParseError> {
    let mut parser = JsonParser {
        input: &mut input.chars().peekable(),
    };
    parser.parse()
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl Value {
    pub fn as_string(&self) -> &String {
        match self {
            Value::String(value) => value,
            _ => panic!(),
        }
    }

    pub fn as_array(&self) -> &Vec<Value> {
        match self {
            Value::Array(array) => array,
            _ => panic!(),
        }
    }

    pub fn as_object(&self) -> &HashMap<String, Value> {
        match self {
            Value::Object(object) => object,
            _ => panic!(),
        }
    }

    pub fn to_string(&self) -> String {
        match &self {
            Value::Null => String::from("null"),
            Value::String(value) => value.clone(),
            Value::Bool(value) => {
                if *value {
                    String::from("true")
                } else {
                    String::from("false")
                }
            }
            Value::Number(number) => match number {
                Number::Integer64(value) => format!("{}", value),
                Number::Float64(value) => format!("{}", value),
            },
            Value::Array(array) => {
                let mut string = String::new();
                string.push('[');
                for i in 0..array.len() {
                    string.push_str(&array[i].to_string());
                    if i != array.len() - 1 {
                        string.push(',');
                    }
                }
                string.push(']');
                string
            }
            Value::Object(object) => {
                let key_count = object.len();
                let mut string = String::new();
                string.push('{');
                let mut i = 0;
                for (name, value) in object.into_iter() {
                    string.push('"');
                    // FIXME: Escape the string.
                    string.push_str(&name);
                    string.push_str("\":\"");
                    string.push_str(&value.to_string());
                    string.push('"');
                    if i != key_count - 1 {
                        string.push(',');
                    }
                    i += 1;
                }
                string.push('}');
                string
            }
        }
    }
}
