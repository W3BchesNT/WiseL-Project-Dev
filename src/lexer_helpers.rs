use std::iter::Peekable;
use std::str::CharIndices;

use crate::lexer::Token;

// Skips the single-line comments
pub fn skip_single_line_comment(chars: &mut Peekable<CharIndices>) -> () {
    // Consume the second slash
    chars.next();
    while let Some(&(_, peeked_char)) = chars.peek() {
        if peeked_char == '\n' { break } else { chars.next(); }
    }
}

// Skips the multi-line comments
pub fn skip_multi_line_comments(chars: &mut Peekable<CharIndices>) -> () {
    // Consume '*', so the loop won't accept '/*/' as a valid comment ('/**/' is valid)
    chars.next();
    // Loop until '*/'
    while let Some((_, next_char)) = chars.next() {
        if next_char != '*' { continue }
        if let Some((_, '/')) = chars.peek() { chars.next(); break }
    }
}

// Converts the whole string literal into a single token and returns it
pub fn handle_string_literals(
    chars: &mut Peekable<CharIndices>,
    initial_char: char,
) -> Token {
    let literal_quote: char = initial_char;
    let mut is_escaped: bool = false;
    let mut string_literal = String::new();
    while let Some((_, next_char)) = chars.next() {
        if is_escaped { is_escaped = false }
        else if next_char == '\\' { is_escaped = true }
        // Do not save a literal quote into the string literal
        else if next_char == literal_quote { break }
        string_literal.push(next_char);
    }

    Token::StringLiteral(string_literal, literal_quote)
}

// Instead of re-constructing brace blocks in the parser,
// we output the brace blocks in lexer as a string (no parsing)
pub fn handle_brace_blocks(chars: &mut Peekable<CharIndices>) -> String {
    // We will push the whole ASM/Directive block to this
    let mut source = String::new();
    let mut depth: usize = 0;
    while let Some(&(_, peeked_char)) = chars.peek() {
        source.push(peeked_char);

        match peeked_char {
            // Skip horizontal whitespaces, carriage returns, and newlines
            ' ' | '\t' | '\r' | '\n' => { chars.next(); },
            // If no first lbrace was found, just ignore the `asm` keyword.
            // Also pop whatever symbol we pushed earlier,
            // so 'asm foo' doesn't become 'Token::ASM(" f")'
            next_char if depth == 0 && next_char != '{' => { source.pop(); return source }

            // Handle both single-line and multi-line comments before handling braces
            '/' => { chars.next(); match chars.peek() {
                // Pop the first slash
                Some((_, '/')) => { source.pop(); skip_single_line_comment(chars); },
                Some((_, '*')) => { source.pop(); skip_multi_line_comments(chars); },

                _              => {},
            } },
            // Handle string literals enclosed in quotes before handling braces.
            // Also consume the first string quote
            '"' | '\'' | '`' => { chars.next(); if let Token::StringLiteral(string_literal, _)
                = handle_string_literals(chars, peeked_char) {
                // Since 'string_literal' comes without the quotes,
                // we use whatever quotes the string used, but the first quote symbol is already
                // added at the start of the loop, so we only add the closing one
                source.push_str(&format!("{string_literal}{peeked_char}"))
            } },

            // If it's the first pair of braces (as in, 'asm { ... }'), then remove them
            // since parser expects the expression inside the braces (even if the said expression
            // contains braces inside)
            '{' => { depth += 1; chars.next(); if depth == 1 { source.pop(); } },
            '}' => { depth -= 1; chars.next(); if depth == 0 { source.pop(); break } },

            _ => { chars.next(); },
        }
    }

    if depth != 0 { unreachable!("Found an invalid brace depth: {depth}") }

    source
}

// Maps the tokens to a string literal
pub fn map_token(token: &Token) -> String {
    let value_string = match token {
        Token::Identifier(value)           => value,
        Token::Bool(value)                 => { return value.to_string() },
        Token::StringLiteral(value, quote) => { return format!("{quote}{value}{quote}") },
        Token::WholeNumber(value)          => value,
        Token::FloatNumber(value)          => value,
        Token::Let                         => "let",
        Token::Mut                         => "mut",
        Token::Static                      => "static",
        Token::Func                        => "func",
        Token::If                          => "if",
        Token::Else                        => "else",
        Token::While                       => "while",
        Token::Break                       => "break",
        Token::Return                      => "return",
        Token::Switch                      => "switch",
        Token::Case                        => "case",
        Token::Match                       => "match",
        Token::Assign                      => "=",
        Token::Div                         => "/",
        Token::Mod                         => "%",
        Token::Dot                         => ".",
        Token::Arrow                       => "->",
        Token::Plus                        => "+",
        Token::Minus                       => "-",
        Token::Star                        => "*",
        Token::Exponent                    => "**",
        Token::PlusPlus                    => "++",
        Token::MinusMinus                  => "--",
        Token::Eq                          => "==",
        Token::InEq                        => "!=",
        Token::GreaterThan                 => ">",
        Token::LessThan                    => "<",
        Token::GreaterThanOrEq             => ">=",
        Token::LessThanOrEq                => "<=",
        Token::Not                         => "!",
        Token::PlusAssign                  => "+=",
        Token::MinusAssign                 => "-=",
        Token::StarAssign                  => "*=",
        Token::DivAssign                   => "/=",
        Token::ExponentAssign              => "**=",
        Token::ModAssign                   => "%=",
        Token::LParen                      => "(",
        Token::RParen                      => ")",
        Token::LBrace                      => "{",
        Token::RBrace                      => "}",
        Token::LBracket                    => "[",
        Token::RBracket                    => "]",
        Token::Colon                       => ":",
        Token::Comma                       => ",",
        Token::Newline                     => "\n",
        Token::Semicolon                   => ";",
        Token::UseLib                      => "UseLib",
        Token::ASM(content)                => { return format!("asm {{ {content} }}") },
        Token::Format                      => "Format",
        Token::Directive(kind)             => kind,
        Token::RawDirective(kind, content) => { return format!("{kind} {{ {content} }}") },
        Token::End                         => "",
        // _                               => unreachable!("Unexpected token: {token:?}"),
    };

    value_string.to_string()
}
