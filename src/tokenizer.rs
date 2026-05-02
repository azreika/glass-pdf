pub trait Tokenizer<T> {
    // Return the token matching the given word or delimiter
    fn token_from_word(&self, word: &str) -> T;
}

pub fn is_identifier_char(c: char) -> bool {
    return c.is_alphanumeric() || matches!(c, '.' | '-' | '+');
}
