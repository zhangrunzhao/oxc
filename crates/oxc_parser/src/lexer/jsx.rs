use super::{Kind, Lexer, Token};
use crate::diagnostics;

use memchr::{memchr, memchr2};
use oxc_syntax::identifier::is_identifier_part;

impl<'a> Lexer<'a> {
    /// `JSXDoubleStringCharacters` ::
    ///   `JSXDoubleStringCharacter` `JSXDoubleStringCharactersopt`
    /// `JSXDoubleStringCharacter` ::
    ///   `JSXStringCharacter` but not "
    /// `JSXSingleStringCharacters` ::
    ///   `JSXSingleStringCharacter` `JSXSingleStringCharactersopt`
    /// `JSXSingleStringCharacter` ::
    ///   `JSXStringCharacter` but not '
    /// `JSXStringCharacter` ::
    ///   `SourceCharacter` but not one of `HTMLCharacterReference`

    /// Read JSX string literal.
    /// # SAFETY
    /// * `delimiter` must be an ASCII character.
    /// * Next char in `lexer.source` must be ASCII.
    pub(super) unsafe fn read_jsx_string_literal(&mut self, delimiter: u8) -> Kind {
        // Skip opening quote
        debug_assert!(delimiter.is_ascii());
        // SAFETY: Caller guarantees next byte is ASCII, so `.add(1)` is a UTF-8 char boundary
        let after_opening_quote = self.source.position().add(1);
        let remaining = self.source.str_from_pos_to_end(after_opening_quote);

        let len = memchr(delimiter, remaining.as_bytes());
        if let Some(len) = len {
            // SAFETY: `after_opening_quote` + `len` is position of delimiter.
            // Caller guarantees delimiter is ASCII, so 1 byte after it is a UTF-8 char boundary.
            let after_closing_quote = after_opening_quote.add(len + 1);
            self.source.set_position(after_closing_quote);
            Kind::Str
        } else {
            self.source.advance_to_end();
            self.error(diagnostics::UnterminatedString(self.unterminated_range()));
            Kind::Undetermined
        }
    }

    pub(crate) fn next_jsx_child(&mut self) -> Token {
        self.token.start = self.offset();
        let kind = self.read_jsx_child();
        self.finish_next(kind)
    }

    /// [`JSXChild`](https://facebook.github.io/jsx/#prod-JSXChild)
    /// `JSXChild` :
    /// `JSXText`
    /// `JSXElement`
    /// `JSXFragment`
    /// { `JSXChildExpressionopt` }
    fn read_jsx_child(&mut self) -> Kind {
        match self.peek() {
            Some('<') => {
                self.consume_char();
                Kind::LAngle
            }
            Some('{') => {
                self.consume_char();
                Kind::LCurly
            }
            Some(_) => {
                // The tokens `{`, `<`, `>` and `}` cannot appear in JSX text.
                // The TypeScript compiler raises the error "Unexpected token. Did you mean `{'>'}` or `&gt;`?".
                // Where as the Babel compiler does not raise any errors.
                // The following check omits `>` and `}` so that more Babel tests can be passed.
                let len = memchr2(b'{', b'<', self.remaining().as_bytes());
                if let Some(len) = len {
                    // SAFETY: `memchr2` guarantees `len` will be offset from current position
                    // of a `{` or `<` byte. So must be a valid UTF-8 boundary, and within bounds of source.
                    let end = unsafe { self.source.position().add(len) };
                    self.source.set_position(end);
                } else {
                    self.source.advance_to_end();
                }
                Kind::JSXText
            }
            None => Kind::Eof,
        }
    }

    /// Expand the current `Ident` token for `JSXIdentifier`
    ///
    /// The current character is at `Ident`, continue reading for `JSXIdentifier` if it has a `-`
    ///
    /// `JSXIdentifier` :
    ///   `IdentifierStart`
    ///   `JSXIdentifier` `IdentifierPart`
    ///   `JSXIdentifier` [no `WhiteSpace` or Comment here] -
    pub(crate) fn continue_lex_jsx_identifier(&mut self) -> Option<Token> {
        if self.peek() != Some('-') {
            return None;
        }
        self.consume_char();
        while let Some(c) = self.peek() {
            if c == '-' || is_identifier_part(c) {
                self.consume_char();
            } else {
                break;
            }
        }
        // Clear the current lookahead `Minus` Token
        self.lookahead.clear();
        Some(self.finish_next(Kind::Ident))
    }
}
