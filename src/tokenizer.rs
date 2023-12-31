use compact_str::{CompactString, ToCompactString};
use malachite::Integer;
use thiserror::Error;

#[derive(Debug, Default, Clone)]
enum TokenizerState {
    #[default]
    Clean,
    Pending(Token),
    InNumber {
        value: Integer,
        radix: u32,
    },
    InOperator(CompactString),
}

#[derive(Debug, Default)]
pub struct Tokenizer {
    state: TokenizerState,
}

impl Tokenizer {
    pub fn update(&mut self, c: char) -> Result<Option<Token>, TokenizeError> {
        use TokenizerState::*;

        match std::mem::take(&mut self.state) {
            Clean => self.state = begin_token(c),
            Pending(token) => {
                self.state = begin_token(c);
                return Ok(Some(token));
            }
            InNumber { mut value, radix } => match c {
                'x' if value == 0 && radix == 8 => self.state = InNumber { value, radix: 16 },
                'b' if value == 0 && radix == 8 => self.state = InNumber { value, radix: 2 },
                '0'..='9' | 'a'..='z' | 'A'..='Z' => {
                    value *= Integer::from(radix);
                    let Some(digit) = c.to_digit(radix) else {
                        return Err(TokenizeError::InvalidNumber);
                    };
                    value += Integer::from(digit);
                    self.state = InNumber { value, radix };
                }
                c => {
                    let token = Token::Val(value);
                    self.state = begin_token(c);
                    return Ok(Some(token));
                }
            },
            InOperator(mut op) => match c {
                '0'..='9' | '+' | '-' | '(' | ')' | 'a'..='z' | 'A'..='Z' => {
                    let token = finalize_operator(op.as_str())
                        .ok_or_else(|| TokenizeError::UnknownOperation(op))?;
                    self.state = begin_token(c);
                    return Ok(Some(token));
                }
                _ if c.is_whitespace() => {
                    let token = finalize_operator(op.as_str())
                        .ok_or_else(|| TokenizeError::UnknownOperation(op))?;
                    self.state = Clean;
                    return Ok(Some(token));
                }
                _ => {
                    op.push(c);
                    self.state = TokenizerState::InOperator(op)
                }
            },
        }
        Ok(None)
    }

    pub fn finalize(&mut self) -> Result<Option<Token>, TokenizeError> {
        use TokenizerState::*;
        match std::mem::take(&mut self.state) {
            Clean => Ok(None),
            Pending(token) => Ok(Some(token)),
            InNumber { value, .. } => Ok(Some(Token::Val(value))),
            InOperator(op) => finalize_operator(op.as_str())
                .ok_or_else(|| TokenizeError::UnknownOperation(op))
                .map(Some),
        }
    }
}

fn begin_token(c: char) -> TokenizerState {
    match c {
        // 0b = binary, 0 = oct, 0x = hex
        '0' => TokenizerState::InNumber {
            value: 0.into(),
            radix: 8,
        },
        '1'..='9' => TokenizerState::InNumber {
            value: (c as u32 - '0' as u32).into(),
            radix: 10,
        },
        '+' => TokenizerState::Pending(Token::Op(Operator::Add)),
        '-' => TokenizerState::Pending(Token::Op(Operator::Sub)),
        '(' => TokenizerState::Pending(Token::ParenOpen),
        ')' => TokenizerState::Pending(Token::ParenClose),
        // Ignore whitespace
        _ if c.is_whitespace() => TokenizerState::Clean,
        _ => TokenizerState::InOperator(c.to_compact_string()),
    }
}

fn finalize_operator(op: &str) -> Option<Token> {
    match op {
        "+" => Some(Token::Op(Operator::Add)),
        "-" => Some(Token::Op(Operator::Sub)),
        "/" => Some(Token::Op(Operator::Div)),
        "(" => Some(Token::ParenOpen),
        ")" => Some(Token::ParenClose),
        "*" => Some(Token::Op(Operator::Mul)),
        "**" => Some(Token::Op(Operator::Pow)),
        _ => None,
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TokenizeError {
    #[error("Invalid number")]
    InvalidNumber,
    #[error("Unknown operation: {0}")]
    UnknownOperation(CompactString),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Val(Value),
    Op(Operator),
    ParenOpen,
    ParenClose,
}

impl From<i64> for Token {
    fn from(value: i64) -> Self {
        Token::Val(value.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

pub type Value = Integer;

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(expr: &str) -> Result<Vec<Token>, TokenizeError> {
        let mut tokens = vec![];
        let mut tokenizer = Tokenizer::default();
        for c in expr.chars() {
            if let Some(token) = tokenizer.update(c)? {
                tokens.push(token)
            }
        }
        if let Some(token) = tokenizer.finalize()? {
            tokens.push(token)
        }
        Ok(tokens)
    }

    #[test]
    fn test_spaces() {
        let result = tokenize(" -  2  +  (  4  )  *    10");
        assert_eq!(
            result,
            Ok(vec![
                Token::Op(Operator::Sub),
                Token::from(2),
                Token::Op(Operator::Add),
                Token::ParenOpen,
                Token::from(4),
                Token::ParenClose,
                Token::Op(Operator::Mul),
                Token::from(10),
            ])
        );
    }

    #[test]
    fn test_sub_negative() {
        let result = tokenize("-12--34");
        assert_eq!(
            result,
            Ok(vec![
                Token::Op(Operator::Sub),
                Token::from(12),
                Token::Op(Operator::Sub),
                Token::Op(Operator::Sub),
                Token::from(34)
            ])
        );
    }

    #[test]
    fn test_parentheses() {
        let result = tokenize("(-2)");
        assert_eq!(
            result,
            Ok(vec![
                Token::ParenOpen,
                Token::Op(Operator::Sub),
                Token::from(2),
                Token::ParenClose
            ])
        );
    }

    #[test]
    fn test_non_decimal() {
        let result = tokenize("0");
        assert_eq!(result, Ok(vec![Token::from(0),]));

        let result = tokenize("123456789");
        assert_eq!(result, Ok(vec![Token::from(123456789),]));

        let result = tokenize("123456789A");
        assert_eq!(result, Err(TokenizeError::InvalidNumber));

        let result = tokenize("0x123456789abcdef");
        assert_eq!(result, Ok(vec![Token::from(0x123456789abcdef),]));

        let result = tokenize("0x123456789abcdefg");
        assert_eq!(result, Err(TokenizeError::InvalidNumber));

        let result = tokenize("0b10");
        assert_eq!(result, Ok(vec![Token::from(0b10),]));

        let result = tokenize("0b102");
        assert_eq!(result, Err(TokenizeError::InvalidNumber));

        let result = tokenize("01234567");
        assert_eq!(result, Ok(vec![Token::from(0o1234567),]));

        let result = tokenize("012345678");
        assert_eq!(result, Err(TokenizeError::InvalidNumber));
    }
}
