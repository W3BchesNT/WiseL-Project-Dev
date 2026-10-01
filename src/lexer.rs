use std::iter::Peekable;
use std::str::CharIndices;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenKind {
    // === 1. Literals & Identifiers (Data) ===
    IDENT, NUMBER, STRING, FORMAT,

    // === 2. Keywords: Declarations & Modifiers ===
    LET, MUT, STATIC, FUNC,

    // === 3. Keywords: Control Flow === 
    IF, ELSE, WHILE, BREAK, RETURN,

    // === 4. Operators ===
    ASSIGN, EQ, PLUSPLUS, DIV, MOD, DOT, ARROW,

    // === 5. Brackets & Delimiters ===
    LPAREN, RPAREN, LBRACE, RBRACE, LBRACKET, RBRACKET,

    // === 6. Punctuation ===
    COLON, COMMA, NEWLINE,

    // === 7. System & Directives ===
    USELIB, ASM, DIRECTIVE,
    END
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
}

// Map keyword string to token type
fn get_keyword_type(word: &str) -> TokenKind {
    match word {
        "func"   => TokenKind::FUNC,
        "if"     => TokenKind::IF,
        "else"   => TokenKind::ELSE,
        "while"  => TokenKind::WHILE,
        "break"  => TokenKind::BREAK,
        "let"    => TokenKind::LET,
        "mut"    => TokenKind::MUT,
        "asm"    => TokenKind::ASM,
        "static" => TokenKind::STATIC,
        "return" => TokenKind::RETURN,
        "Format" => TokenKind::FORMAT,
        "UseLib" => TokenKind::USELIB,
        _        => TokenKind::IDENT,
    }
}

// Convert source code string into token vector using iterators
pub fn tokenize(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = source.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        // Skip horizontal whitespaces and carriage returns
        if ch == ' ' || ch == '\t' || ch == '\r' {
            continue;
        }

        // Handle newlines as statement terminators
        if ch == '\n' {
            tokens.push(Token {
                kind: TokenKind::NEWLINE,
                value: String::from("\n"),
            });
            continue;
        }

        // Handle single-line comments starting with '//'
        if ch == '/' {
            if let Some(&(_, '/')) = chars.peek() {
                while let Some(&(_, c)) = chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    chars.next();
                }
                continue;
            }
        }

        // Handle negative numbers (e.g., -123 or -42h)
        if ch == '-' {
            if let Some(&(_, next_ch)) = chars.peek() {
                if next_ch.is_ascii_digit() {
                    let mut num = String::from("-");
                    while let Some(&(_, c)) = chars.peek() {
                        if c.is_ascii_digit() || c == 'h' || c == 'H' {
                            num.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token {
                        kind: TokenKind::NUMBER,
                        value: num,
                    });
                    continue;
                }
            }
        }

        // Handle double-character operators
        if ch == '-' && chars.peek() == Some(&(i + 1, '>')) {
            chars.next();
            tokens.push(Token { kind: TokenKind::ARROW, value: String::from("->") });
            continue;
        }
        if ch == '=' && chars.peek() == Some(&(i + 1, '=')) {
            chars.next();
            tokens.push(Token { kind: TokenKind::EQ, value: String::from("==") });
            continue;
        }
        if ch == '+' && chars.peek() == Some(&(i + 1, '+')) {
            chars.next();
            tokens.push(Token { kind: TokenKind::PLUSPLUS, value: String::from("++") });
            continue;
        }

        // Handle directives starting with '@'
        if ch == '@' {
            let mut word = String::from("@");
            while let Some(&(_, c)) = chars.peek() {
                if c.is_alphanumeric() || c == '_' || c == '.' {
                    word.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(Token {
                kind: TokenKind::DIRECTIVE,
                value: word,
            });
            continue;
        }

        // Handle string literals enclosed in quotes ("..." or '...')
        if ch == '"' || ch == '\'' {
            let quote = ch;
            let mut s = String::from(quote);
            let mut escaped = false;
            while let Some(&(_, c)) = chars.peek() {
                chars.next();
                s.push(c);
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                } else if c == quote {
                    break;
                }
            }
            tokens.push(Token {
                kind: TokenKind::STRING,
                value: s,
            });
            continue;
        }

        // Handle numeric literals (with optional hex suffix h/H)
        if ch.is_ascii_digit() {
            let mut num = String::from(ch);
            while let Some(&(_, c)) = chars.peek() {
                if c.is_ascii_digit() || c == 'h' || c == 'H' {
                    num.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(Token {
                kind: TokenKind::NUMBER,
                value: num,
            });
            continue;
        }

        // Handle identifiers and keywords
        if ch.is_ascii_alphabetic() || ch == '_' {
            let mut word = String::from(ch);
            while let Some(&(_, c)) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    word.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            let kind = get_keyword_type(&word);
            tokens.push(Token { kind, value: word });
            continue;
        }

        // Handle single-character delimiters, punctuation, and fallback operators
        let (kind, val) = match ch {
            '(' => (TokenKind::LPAREN, "("),
            ')' => (TokenKind::RPAREN, ")"),
            '{' => (TokenKind::LBRACE, "{"),
            '}' => (TokenKind::RBRACE, "}"),
            '[' => (TokenKind::LBRACKET, "["),
            ']' => (TokenKind::RBRACKET, "]"),
            ',' => (TokenKind::COMMA, ","),
            '=' => (TokenKind::ASSIGN, "="),
            '/' => (TokenKind::DIV, "/"),
            '%' => (TokenKind::MOD, "%"),
            '.' => (TokenKind::DOT, "."),
            ':' => (TokenKind::COLON, ":"),
            _   => (TokenKind::IDENT, &source[i..i + ch.len_utf8()]),
        };

        tokens.push(Token {
            kind,
            value: val.to_string(),
        });
    }

    // Append end marker token
    tokens.push(Token {
        kind: TokenKind::END,
        value: String::new(),
    });

    tokens
}