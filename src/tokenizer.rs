pub trait Tokenizer<T> {
    // Return the token matching the given word or delimiter
    fn token_from_word(&self, word: &str) -> T;

    // Return the next byte in the stream without moving forward
    fn peek_u8(&self) -> u8;

    fn peek(&self) -> char {
        return self.peek_u8() as char;
    }

    fn peek_is(&self, c: char) -> bool {
        return self.peek() == c;
    }
}

pub fn is_identifier_char(c: char) -> bool {
    return c.is_alphanumeric() || matches!(c, '.' | '-' | '+');
}
