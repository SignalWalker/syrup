use std::borrow::Cow;

use crate::de::{Cursor, DecodeError, LexError, TokenTree};

#[derive(Debug, Clone)]
pub struct TokenStream<'input> {
    tokens: Vec<TokenTree<'input>>,
}

impl<'input> From<TokenTree<'input>> for TokenStream<'input> {
    fn from(tree: TokenTree<'input>) -> Self {
        Self { tokens: vec![tree] }
    }
}

impl<'input> TokenStream<'input> {
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn as_slice(&self) -> &[TokenTree<'input>] {
        self.tokens.as_slice()
    }

    pub fn peek(&self) -> Option<&TokenTree<'input>> {
        self.tokens.first()
    }

    pub fn pop(&mut self) -> Option<TokenTree<'input>> {
        self.tokens.pop()
    }

    pub fn insert(&mut self, index: usize, element: TokenTree<'input>) {
        self.tokens.insert(index, element);
    }

    pub fn require(
        &mut self,
        expected: Cow<'input, str>,
    ) -> Result<TokenTree<'input>, DecodeError<'input>> {
        self.pop().ok_or(DecodeError::missing(expected))
    }

    pub const fn new(tokens: Vec<TokenTree<'input>>) -> Self {
        Self { tokens }
    }

    /// Lex all tokens from an input slice and return the lexed tokens, the remaining input, and, if
    /// not all input could be parsed, an error.
    pub fn tokenize(
        mut input: Cursor<'input>,
    ) -> (TokenStream<'input>, Cursor<'input>, Option<LexError>) {
        let mut res: Vec<TokenTree<'_>> = Vec::new();
        loop {
            if input.rem.is_empty() {
                return (Self::new(res), input, None);
            }
            match TokenTree::tokenize(input) {
                Ok((tree, rest)) => {
                    input = rest;
                    res.push(tree);
                }
                Err(error) => {
                    return (Self::new(res), input, Some(error));
                }
            }
        }
    }
}

impl<'i> IntoIterator for TokenStream<'i> {
    type Item = TokenTree<'i>;
    type IntoIter = <Vec<TokenTree<'i>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}
