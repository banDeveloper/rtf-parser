use crate::tokens::{ControlWord, Token};
use crate::utils::StrUtils;

use std::fmt;
use crate::{recursive_tokenize, recursive_tokenize_with_init};

pub enum LexerError {
    Error(String),
    InvalidLastChar,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = write!(f, "[RTF Lexer] : ");
        let _ = match self {
            LexerError::InvalidLastChar => write!(f, "Invalid last char, should be '}}'"),
            LexerError::Error(msg) => write!(f, "{}", msg),
        };
        return Ok(());
    }
}

impl fmt::Debug for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return write!(f, "{}", self);
    }
}

pub struct Lexer;

impl Lexer {
    pub fn scan(src: &str) -> Result<Vec<Token>, LexerError> {
        let src = src.trim(); // Sanitize src : Trim the leading whitespaces
        let mut it = src.chars();
        let mut tokens: Vec<Token> = vec![];
        let mut slice_start_index = 0;
        let mut current_index = 0;
        let mut previous_char = ' ';
        while let Some(c) = it.next() {
            match c {
                // TODO: Handle char over code 127 for escaped chars
                // Handle Escaped chars : "\" + any charcode below 127
                '{' | '}' | '\\' | '\n' if previous_char == '\\' => {}
                '{' | '}' | '\\' | '\n' => {
                    // End of slice chars
                    if slice_start_index < current_index {
                        // Close slice
                        let slice = &src[slice_start_index..current_index];
                        // Get the corresponding token(s)
                        let slice_tokens = Self::tokenize(slice)?;
                        for slice_token in slice_tokens {
                            tokens.push(slice_token);
                        }
                        slice_start_index = current_index;
                    }
                }
                // Others chars
                _ => {}
            }
            current_index += 1;
            previous_char = c;
        }
        // Manage last token (should always be "}")
        // TODO: Handle the case where the last char is a line return
        if slice_start_index < current_index {
            let slice = &src[slice_start_index..current_index];
            if slice != "}" {
                return Err(LexerError::InvalidLastChar);
            }
            tokens.push(Token::ClosingBracket);
        }
        return Ok(tokens);
    }

    /// Get a string slice cut but the scanner and return the coreesponding token(s)
    fn tokenize(slice: &str) -> Result<Vec<Token>, LexerError> {
        let mut starting_chars = slice.trim_matches(' ').chars().take(2);
        return match (starting_chars.next(), starting_chars.next()) {
            // If it starts with \ : escaped text or control word
            (Some('\\'), Some(c)) => match c {
                '{' | '}' | '\\' => {
                    // Handle escaped chars
                    let tail = slice.get(1..).unwrap_or("");
                    return Ok(vec![Token::PlainText(tail)]); // No recursive tokenize here, juste some plain text because the char is escaped
                }
                '\n' => {
                    // CRLF
                    let mut ret = vec![Token::CRLF];
                    if let Some(tail) = slice.get(2..) {
                        recursive_tokenize!(tail, ret);
                    }
                    return Ok(ret);
                }
                'a'..='z' => {
                    // Identify control word
                    // ex: parse "\b Words in bold" -> (Token::ControlWord(ControlWord::Bold), Token::ControlWordArgument("Words in bold")
                    let (mut ident, tail) = slice.split_first_whitespace();
                    // if ident end with semicolon, strip it for correct value parsing
                    ident = if ident.chars().last().unwrap_or(' ') == ';' { &ident[0..ident.len() - 1] } else { ident };
                    let control_word = ControlWord::from(ident)?;
                    let mut ret = vec![Token::ControlSymbol(control_word)];
                    recursive_tokenize!(tail, ret);
                    return Ok(ret);
                }
                '*' => Ok(vec![Token::IgnorableDestination]),
                _ => Ok(vec![]),
            },
            (Some('\n'), Some(_)) => recursive_tokenize!(&slice[1..]), // Ignore the CRLF if it's not escaped
            // Handle brackets
            (Some('{'), None) => Ok(vec![Token::OpeningBracket]),
            (Some('}'), None) => Ok(vec![Token::ClosingBracket]),
            (Some('{'), Some(_)) => recursive_tokenize_with_init!(Token::OpeningBracket, &slice[1..]),
            (Some('}'), Some(_)) => recursive_tokenize_with_init!(Token::ClosingBracket, &slice[1..]),
            (None, None) => Err(LexerError::Error(format!("Empty token {}", &slice))),
            // Else, it's plain text
            _ => {
                let text = slice.trim();
                if text == "" { return Ok(vec![]); }
                return Ok(vec![Token::PlainText(slice)]);
            }
        };
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::ControlWord::Par;
    use crate::{include_test_file, Parser};
    use crate::lexer::Lexer;
    use crate::tokens::ControlWord::{Ansi, Bold, FontNumber, FontSize, FontTable, Rtf, Pard, Unknown};
    use crate::tokens::Property::*;
    use crate::tokens::Token::*;

    #[test]
    fn simple_tokenize_test() {
        let tokens = Lexer::tokenize(r"\b Words in bold").unwrap();
        assert_eq!(tokens, vec![ControlSymbol((Bold, None)), PlainText("Words in bold"),]);
    }

    #[test]
    fn scan_entire_file_test() {
        let tokens = Lexer::scan(r#"{ \rtf1\ansi{\fonttbl\f0\fswiss Helvetica;}\f0\pard Voici du texte en {\b gras}.\par }"#);
        assert_eq!(
            tokens.unwrap(),
            vec![
                OpeningBracket,
                ControlSymbol((Rtf, Value(1))),
                ControlSymbol((Ansi, None)),
                OpeningBracket,
                ControlSymbol((FontTable, None)),
                ControlSymbol((FontNumber, Value(0))),
                ControlSymbol((Unknown("\\fswiss"), None)),
                PlainText("Helvetica;"),
                ClosingBracket,
                ControlSymbol((FontNumber, Value(0))),
                ControlSymbol((Pard, None)),
                PlainText("Voici du texte en "),
                OpeningBracket,
                ControlSymbol((Bold, None)),
                PlainText("gras"),
                ClosingBracket,
                PlainText("."),
                ControlSymbol((Par, None)),
                ClosingBracket,
            ]
        );
    }

    #[test]
    fn scan_escaped_text() {
        let tokens = Lexer::scan(
            r#"\f0\fs24 \cf0 test de code \
if (a == b) \{\
    test();\
\} else \{\
    return;\
\}}"#,
        );
        assert_eq!(
            tokens.unwrap(),
            vec![
                ControlSymbol((FontNumber, Value(0))),
                ControlSymbol((FontSize, Value(24))),
                ControlSymbol((Unknown("\\cf"), Value(0))),
                PlainText("test de code "),
                CRLF,
                PlainText("if (a == b) "),
                PlainText("{"),
                CRLF,
                PlainText("    test();"),
                CRLF,
                PlainText("} else "),
                PlainText("{"),
                CRLF,
                PlainText("    return;"),
                CRLF,
                PlainText("}"),
                ClosingBracket
            ],
        );
    }

    #[test]
    fn scan_ignorable_destination() {
        let text = r"{\*\expandedcolortbl;;}";
        let tokens = Lexer::scan(text);
        assert_eq!(
            tokens.unwrap(),
            vec![OpeningBracket, IgnorableDestination, ControlSymbol((Unknown(r"\expandedcolortbl;"), None)), ClosingBracket,]
        )
    }

    #[test]
    fn should_parse_control_symbol_ending_semicolon() {
        let text = r"{\red255\blue255;}";
        let tokens = Lexer::scan(text);
        assert_eq!(
            tokens.unwrap(),
            vec![OpeningBracket, ControlSymbol((Unknown(r"\red"), Value(255))), ControlSymbol((Unknown(r"\blue"), Value(255))), ClosingBracket]
        );
    }

    #[test]
    fn lex_with_leading_whitespaces() {
        // Try to parse without error
        let rtf_content = "\t {\\rtf1 }\n "; // Not raw str for the whitespace to be trimed
        let tokens = Lexer::scan(rtf_content).unwrap();
        assert_eq!(tokens, vec![OpeningBracket, ControlSymbol((Rtf, Value(1))), ClosingBracket]);
    }

    #[test]
    fn should_parse_line_return() {
        // From Microsoft's reference: "A carriage return (character value 13) or linefeed (character value 10)
        // will be treated as a \par control if the character is preceded by a backslash.
        // You must include the backslash; otherwise, RTF ignores the control word."
        let text = r#"{\partightenfactor0

\fs24 \cf0 Font size 12,
\f0\b bold text. \ul Underline,bold text.\
 }"#;
        let tokens = Lexer::scan(text).unwrap();
        assert_eq!(tokens, [
            OpeningBracket,
            ControlSymbol((Unknown("\\partightenfactor"), Value(0))),
            ControlSymbol((FontSize, Value(24))),
            ControlSymbol((Unknown("\\cf"), Value(0))),
            PlainText("Font size 12,"),
            ControlSymbol((FontNumber, Value(0))),
            ControlSymbol((Bold, None)),
            PlainText("bold text. "),
            ControlSymbol((Unknown("\\ul"), None)),
            PlainText("Underline,bold text."),
            CRLF,
            ClosingBracket
        ]);
    }
}
