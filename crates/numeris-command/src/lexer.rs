//! # Lexer
//!
//! Tokenizer for the Numeris command language. Handles identifiers,
//! numbers, strings, punctuation and `//` comments.

use numeris_core::error::{NumerisError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Number(f64),
    Str(String),
    Comma,
    LParen,
    RParen,
    Eq,    // '=' (also '==' collapsed)
    NotEq, // '!='
    Gt,
    Ge,
    Lt,
    Le,
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub pos: usize,
}

pub fn lex(source: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    let n = chars.len();
    while i < n {
        let c = chars[i];
        // Whitespace.
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // Line comments: // or * at the start of a (trimmed) line.
        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '*' {
            let line_start = source[..source.char_indices().nth(i).map(|(b, _)| b).unwrap_or(0)]
                .rfind('\n')
                .map(|p| p + 1)
                .unwrap_or(0);
            let prefix = &source.as_bytes()
                [line_start..source.char_indices().nth(i).map(|(b, _)| b).unwrap_or(0)];
            if prefix.iter().all(|b| b.is_ascii_whitespace()) {
                while i < n && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
        }
        let pos = i;
        match c {
            ',' => {
                tokens.push(Token {
                    tok: Tok::Comma,
                    pos,
                });
                i += 1;
            }
            '(' => {
                tokens.push(Token {
                    tok: Tok::LParen,
                    pos,
                });
                i += 1;
            }
            ')' => {
                tokens.push(Token {
                    tok: Tok::RParen,
                    pos,
                });
                i += 1;
            }
            '+' => {
                tokens.push(Token {
                    tok: Tok::Plus,
                    pos,
                });
                i += 1;
            }
            '-' => {
                tokens.push(Token {
                    tok: Tok::Minus,
                    pos,
                });
                i += 1;
            }
            '*' => {
                tokens.push(Token {
                    tok: Tok::Star,
                    pos,
                });
                i += 1;
            }
            '/' => {
                tokens.push(Token {
                    tok: Tok::Slash,
                    pos,
                });
                i += 1;
            }
            '^' => {
                tokens.push(Token {
                    tok: Tok::Caret,
                    pos,
                });
                i += 1;
            }
            '=' => {
                if i + 1 < n && chars[i + 1] == '=' {
                    i += 2;
                } else {
                    i += 1;
                }
                tokens.push(Token { tok: Tok::Eq, pos });
            }
            '!' => {
                if i + 1 < n && chars[i + 1] == '=' {
                    tokens.push(Token {
                        tok: Tok::NotEq,
                        pos,
                    });
                    i += 2;
                } else {
                    return Err(NumerisError::syntax(
                        format!("Unexpected character '!' at position {}.", pos),
                        "An exclamation mark is only valid as part of '!=' in a condition.",
                        "Use '!=' to express 'not equal', for example: drop if wage != 0.",
                    ));
                }
            }
            '>' => {
                if i + 1 < n && chars[i + 1] == '=' {
                    tokens.push(Token { tok: Tok::Ge, pos });
                    i += 2;
                } else {
                    tokens.push(Token { tok: Tok::Gt, pos });
                    i += 1;
                }
            }
            '<' => {
                if i + 1 < n && chars[i + 1] == '=' {
                    tokens.push(Token { tok: Tok::Le, pos });
                    i += 2;
                } else {
                    tokens.push(Token { tok: Tok::Lt, pos });
                    i += 1;
                }
            }
            '"' => {
                // String literal.
                let mut s = String::new();
                i += 1;
                while i < n && chars[i] != '"' {
                    s.push(chars[i]);
                    i += 1;
                }
                if i >= n {
                    return Err(NumerisError::syntax(
                        "A quoted string was not closed.",
                        "The command contains a double quote with no matching closing quote.",
                        "Check the quotation marks in the command and try again.",
                    ));
                }
                i += 1;
                tokens.push(Token {
                    tok: Tok::Str(s),
                    pos,
                });
            }
            '0'..='9' => {
                let start = i;
                // Scan a numeric literal (digits, optional dot, scientific
                // notation).
                while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                if i < n && (chars[i] == 'e' || chars[i] == 'E') {
                    let mut j = i + 1;
                    if j < n && (chars[j] == '+' || chars[j] == '-') {
                        j += 1;
                    }
                    if j < n && chars[j].is_ascii_digit() {
                        i = j;
                        while i < n && chars[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                }
                // If letters immediately follow the numeric run (as in the
                // estimator name "2sls"), the whole token is an identifier.
                if i < n && chars[i].is_alphabetic() {
                    while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    let text: String = chars[start..i].iter().collect();
                    tokens.push(Token {
                        tok: Tok::Ident(text),
                        pos: start,
                    });
                } else {
                    let text: String = chars[start..i].iter().collect();
                    let value: f64 = text.parse().map_err(|_| {
                        NumerisError::syntax(
                            format!("Could not read the number '{text}'."),
                            "The token looks like a number but is not a valid numeric literal.",
                            "Check for a misplaced decimal point or 'e' in the command.",
                        )
                    })?;
                    tokens.push(Token {
                        tok: Tok::Number(value),
                        pos: start,
                    });
                }
            }
            _ if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                tokens.push(Token {
                    tok: Tok::Ident(text),
                    pos: start,
                });
            }
            _ => {
                return Err(NumerisError::syntax(
                    format!("Unexpected character '{c}' at position {}.", pos),
                    "The command contains a character that is not part of the Numeris command language.",
                    "Check the command syntax with 'help' and try again.",
                ));
            }
        }
    }
    tokens.push(Token {
        tok: Tok::Eof,
        pos: n,
    });
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_regression_command() {
        let toks = lex("regress wage education experience female, robust").unwrap();
        assert_eq!(toks[0].tok, Tok::Ident("regress".into()));
        assert_eq!(toks[5].tok, Tok::Comma);
        assert_eq!(toks[6].tok, Tok::Ident("robust".into()));
        assert_eq!(toks.last().unwrap().tok, Tok::Eof);
    }

    #[test]
    fn lexes_numbers() {
        let toks = lex("ttest wage == 2.5e2").unwrap();
        assert_eq!(toks[3].tok, Tok::Number(250.0));
    }

    #[test]
    fn lexes_strings_and_operators() {
        let toks = lex(r#"use "my data.csv""#).unwrap();
        assert_eq!(toks[1].tok, Tok::Str("my data.csv".into()));
        let toks = lex("drop if x >= 3").unwrap();
        assert_eq!(toks[3].tok, Tok::Ge);
    }

    #[test]
    fn comments_are_ignored() {
        let toks = lex("regress y x // trailing comment\nsummarize y // other").unwrap();
        assert_eq!(toks[3].tok, Tok::Ident("summarize".into()));
    }

    #[test]
    fn star_comment_only_at_line_start() {
        let toks = lex("* whole line comment\nregress y x").unwrap();
        assert_eq!(toks[0].tok, Tok::Ident("regress".into()));
        // Multiplication inside expressions is preserved.
        let toks = lex("generate z = x * 2").unwrap();
        assert_eq!(toks[4].tok, Tok::Star);
    }

    #[test]
    fn unclosed_string_is_a_syntax_error() {
        let err = lex(r#"use "oops"#).unwrap_err();
        assert!(err.what.contains("not closed"));
    }
}
