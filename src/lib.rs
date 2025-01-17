// RTF parser for Text Editor
// Support RTF version 1.9.1
// specification is available here : https://dokumen.tips/documents/rtf-specification.html
// Miscroft specification is : https://learn.microsoft.com/en-us/previous-versions/office/office-10/aa140283(v=office.10)

#![allow(irrefutable_let_patterns)]

mod document;
mod tokens;
mod lexer;
mod parser;
mod header;
mod utils;

// Public API of the crate
pub use crate::header::{CharacterSet, RtfHeader, FontFamily};
pub use crate::lexer::{Lexer, LexerError};
pub use crate::parser::{Painter, Parser, ParserError, StyleBlock};
pub use crate::document::RtfDocument;
pub use crate::tokens::{Token, ControlSymbol, ControlWord, Property};