use std::vec::IntoIter;

use super::{word::WordTokenizer, Token};

pub struct JapaneseTokenizer<'x> {
    word_tokenizer: WordTokenizer<'x>,
    tokens: IntoIter<String>,
    token_offset: usize,
    token_len: usize,
    token_len_cur: usize,
    max_token_length: usize,
}

impl<'x> JapaneseTokenizer<'x> {
    pub fn new(text: &str, max_token_length: usize) -> JapaneseTokenizer {
        JapaneseTokenizer {
            word_tokenizer: WordTokenizer::new(text),
            tokens: Vec::new().into_iter(),
            max_token_length,
            token_offset: 0,
            token_len: 0,
            token_len_cur: 0,
        }
    }
}

impl<'x> Iterator for JapaneseTokenizer<'x> {
    type Item = Token<'x>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(jp_token) = self.tokens.next() {
                let offset_start = self.token_offset + self.token_len_cur;
                self.token_len_cur += jp_token.len();

                if jp_token.len() <= self.max_token_length {
                    return Token::new(offset_start, jp_token.len(), jp_token.into()).into();
                }
            } else {
                let token = self.word_tokenizer.next()?;
                self.tokens = tinysegmenter::tokenize(token.word.as_ref()).into_iter();
                self.token_offset = token.offset as usize;
                self.token_len = token.len as usize;
                self.token_len_cur = 0;
            }
        }
    }
}
