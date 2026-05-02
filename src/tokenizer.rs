use std::str::FromStr;

pub trait Tokenizer<T> {
    // Return the token matching the given word or delimiter
    fn token_from_word(&self, word: &str) -> T;

    // Return the next byte in the stream without moving forward
    fn peek_u8(&self) -> u8;

    // Move one step forward
    fn step_ahead(&mut self);

    // ------------------- //

    fn peek(&self) -> char {
        return self.peek_u8() as char;
    }

    fn peek_is(&self, c: char) -> bool {
        return self.peek() == c;
    }

    fn lex_char(&mut self) -> char {
        let result = self.peek();
        self.step_ahead();
        return result;
    }

    fn eat_char(&mut self, c: char) {
        let cc = self.lex_char();
        assert_eq!(cc ,c);
    }

    // Lex the next word or delimiter in the byte sequence
    fn lex_word(&mut self) -> String {
        if !is_identifier_char(self.peek()) {
            return self.lex_char().to_string();
        }
        let mut chars = vec![];
        while is_identifier_char(self.peek()) {
            chars.push(self.lex_char());
        }
        let str = chars.iter().collect();
        return str;
    }

    fn lex_number<N>(&mut self) -> N where
        N: FromStr,
        N::Err: std::fmt::Debug,
    {
        let mut chars = vec![];
        if self.peek_is('-') {
            chars.push(self.lex_char());
        }
        let mut cc = self.peek();
        while cc == '.' || cc.is_numeric() {
            chars.push(self.lex_char());
            cc = self.peek();
        }
        let str: String = chars.iter().collect();
        return str.parse().unwrap();
    }
}

fn is_identifier_char(c: char) -> bool {
    return c.is_alphanumeric() || matches!(c, '.' | '-' | '+' | '_');
}
