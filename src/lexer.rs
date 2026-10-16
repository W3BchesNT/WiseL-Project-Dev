use std::iter::Peekable;
use std::str::CharIndices;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    // === 1. Literals & Identifiers (Data) ===
    Identifier(String), Bool(bool), StringLiteral(String),
    // BTW I named this as "WholeNumber" instead of "Integer"
    // since we don't detect negative numbers here. We emit a negative sign and a number,
    // then the parser handles the negative numbers
    WholeNumber(String), FloatNumber(String),

    // === 2. Keywords: Declarations & Modifiers ===
    Let, Mut, Static, Func,

    // === 3. Keywords: Control Flow === 
    If, Else, While, Break, Return, Switch, Case, Match,

    // === 4. Operators ===
    Assign, Div, Mod, Dot, Arrow, Plus, Minus, Star, Exponent, PlusPlus, MinusMinus,
    Eq, InEq, GreaterThan, LessThan, GreaterThanOrEq, LessThanOrEq, Not,
    PlusAssign, MinusAssign, StarAssign, DivAssign, ExponentAssign, ModAssign,

    // === 5. Brackets & Delimiters ===
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,

    // === 6. Punctuation ===
    Colon, Comma, Newline, Semicolon,

    // === 7. System & Directives ===
    UseLib, ASM, Format, Directive(String),

    End,
}

fn identify_char_sequence(word: String) -> Token {
    match word.as_str() {
        "true"   => Token::Bool(true),
        "false"  => Token::Bool(false),

        "let"    => Token::Let,
        "mut"    => Token::Mut,
        "static" => Token::Static,
        "fn" | "fun" | "func" | "function"
                 => Token::Func,

        "if"     => Token::If,
        "else"   => Token::Else,
        "while"  => Token::While,
        "break"  => Token::Break,
        "return" => Token::Return,
        "switch" => Token::Switch,
        "case"   => Token::Case,
        "match"  => Token::Match,

        "asm"    => Token::ASM,
        "UseLib" => Token::UseLib,
        "Format" => Token::Format,

        _        => Token::Identifier(word),
    }
}

// Identifies '+', '--', '**', '/=', '**=', '->', etc.
fn identify_double_operator(chars: &mut Peekable<CharIndices>, current_char: char) -> Token {
    let peeked_char: char = chars
        .peek()
        .map(|&(_, actual_char)| actual_char)
        // A placeholder
        .unwrap_or('\0');

    match (current_char, peeked_char) {
        // Let both '->' and '=>' indicate an arrow
        ('-', '>') |
        ('=', '>') => { chars.next(); Token::Arrow },

        // Comparison
        ('=', '=') => { chars.next(); Token::Eq },
        ('!', '=') => { chars.next(); Token::InEq },
        ('>', '=') => { chars.next(); Token::GreaterThanOrEq },
        ('<', '=') => { chars.next(); Token::LessThanOrEq },
        ('>', _) => Token::GreaterThan,
        ('<', _) => Token::LessThan,

        // Mathematical operators
        ('=', _)   => Token::Assign,
        ('!', _)   => Token::Not,
        ('+', '+') => { chars.next(); Token::PlusPlus },
        ('+', '=') => { chars.next(); Token::PlusAssign },
        ('+', _)   => Token::Plus,
        ('-', '-') => { chars.next(); Token::MinusMinus },
        ('-', '=') => { chars.next(); Token::MinusAssign },
        ('-', _)   => Token::Minus,
        // Can represent '**=', so it's a complex case
        ('*', '*') => {
            chars.next();
            if let Some((_, '=')) = chars.peek() {
                chars.next();
                return Token::ExponentAssign;
            }

            Token::Exponent
        },
        ('*', '=') => { chars.next(); Token::StarAssign },
        // No C-styled pointers, I guess?
        ('*', _)   => Token::Star,
        ('/', '=') => { chars.next(); Token::DivAssign },
        ('/', _)   => Token::Div,
        ('%', '=') => { chars.next(); Token::ModAssign },
        ('%', _)   => Token::Mod,

        _ => unreachable!(
            "Invalid mathematical operator: {}",
            String::from_iter([current_char, peeked_char]),
        ),
    }
}

// Convert the source code string into a token vector using iterators
pub fn tokenize(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = source.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        match ch {
            // Skip horizontal whitespaces and carriage returns
            ' ' | '\t' | '\r' => { continue },

            // Handle newlines and semicolons as statement terminators
            '\n' => { tokens.push(Token::Newline) },
            ';'  => { tokens.push(Token::Semicolon) },

            // Handle both single-line and multi-line comments
            '/' => match chars.peek() {
                Some((_, '/')) => {
                    // Consume the second slash
                    chars.next();
                    while let Some((_, next_char)) = chars.next() {
                        if next_char == '\n' {
                            tokens.push(Token::Newline);
                            break;
                        }
                    }
                },
                Some((_, '*')) => {
                    // Consume '*', so the loop won't accept '/*/' as a valid comment ('/**/' is valid)
                    chars.next();
                    // Loop until '*/'
                    while let Some((_, next_char)) = chars.next() {
                        if next_char != '*' { continue }
                        if let Some((_, '/')) = chars.peek() { chars.next(); break }
                    }
                },
                _ => {
                    /* Regex maybe? https://crates.io/crates/regex */

                    // Operators '/', '/='
                    tokens.push(identify_double_operator(&mut chars, ch));
                },
            },

            '+' | '-' | '*' | '%' |
            '=' | '!' |
            '>' | '<' => { tokens.push(identify_double_operator(&mut chars, ch)); },

            // Handle directives starting with a '@' symbol
            '@' => {
                let mut directive_value = String::from(ch);
                while let Some(&(_, peeked_char)) = chars.peek() {
                    if !peeked_char.is_alphanumeric() && peeked_char != '_' && peeked_char != '.' { break }

                    chars.next();
                    directive_value.push(peeked_char);
                }
                tokens.push(Token::Directive(directive_value));
            },

            // Handle string literals enclosed in quotes ("...", '...', `...`)
            '"' | '\'' | '`' => {
                let literal_quote: char = ch;
                let mut is_escaped: bool = false;
                let mut string_literal = String::new();
                while let Some((_, next_char)) = chars.next() {
                    if is_escaped { is_escaped = false }
                    else if next_char == '\\' { is_escaped = true }
                    // Do not save a literal quote into the string literal
                    else if next_char == literal_quote { break }
                    string_literal.push(next_char);
                }
                tokens.push(Token::StringLiteral(string_literal));
            },

            // Handle number literals (with an optional hex suffix 'h'/'H')
            number_char if number_char.is_ascii_digit() => {
                let mut has_dot: bool = false;
                let mut number_literal = String::from(number_char);
                while let Some(&(_, peeked_char)) = chars.peek() {
                    match peeked_char {
                        'H' |
                        'h' => {
                            chars.next();
                            number_literal.push(peeked_char);
                            break;
                        },

                        '.' => {
                            if has_dot { break } else {
                                chars.next();
                                number_literal.push(peeked_char);
                                has_dot = true;
                            }
                        },

                        // Handle both HEX and base 10 digits. If the HEX digits are present,
                        // but the number lacks a 'h' symbol, the token will still be emitted,
                        // so the parser should handle the rest that is going to happen
                        digit if digit.is_ascii_hexdigit() => {
                            chars.next();
                            number_literal.push(digit);
                        },

                        _ => { break },
                    }
                }
                tokens.push(
                    if has_dot { Token::FloatNumber(number_literal) }
                    else { Token::WholeNumber(number_literal) }
                );
            }

            // Handle single-character delimiters and punctuation
            '(' => { tokens.push(Token::LParen) },
            ')' => { tokens.push(Token::RParen) },
            '{' => { tokens.push(Token::LBrace) },
            '}' => { tokens.push(Token::RBrace) },
            '[' => { tokens.push(Token::LBracket) },
            ']' => { tokens.push(Token::RBracket) },
            ',' => { tokens.push(Token::Comma) },
            '.' => { tokens.push(Token::Dot) },
            ':' => { tokens.push(Token::Colon) },

            // Handle identifiers and keywords
            identifier_char if
                identifier_char.is_ascii_alphabetic() ||
                identifier_char == '_' ||
                identifier_char == '$'
            => {
                let mut identifier_literal = String::from(ch);
                while let Some(&(_, peeked_char)) = chars.peek() {
                    // Identifiers must always start with an alphabetic symbol,
                    // but they may contain numbers, '_', or '$', like in JavaScript
                    if !peeked_char.is_ascii_alphanumeric() && peeked_char != '_' && peeked_char != '$' { break }
                    chars.next();
                    identifier_literal.push(peeked_char);
                }
                tokens.push(identify_char_sequence(identifier_literal));
            },

            _ => {
                /*
                 * Ignore or not to ignore?
                 * Characters inside strings and comments are already handled
                 */
                unreachable!("Found an invalid character while tokenizing: {}", String::from(ch));
            },
        }
    }

    // Append the end marker token
    tokens.push(Token::End);

    tokens
}