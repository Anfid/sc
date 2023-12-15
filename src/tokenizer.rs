use compact_str::{CompactString, ToCompactString};
use thiserror::Error;

#[derive(Debug, Default)]
enum TokenizerState {
    #[default]
    Clean,
    InNumber {
        value: i64,
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

        match self.state {
            Clean => {
                self.state = begin_token(c);
                Ok(None)
            }
            InNumber { mut value, radix } => match c {
                'x' if value == 0 && radix == 8 => {
                    self.state = InNumber { value, radix: 16 };
                    Ok(None)
                }
                'b' if value == 0 && radix == 8 => {
                    self.state = InNumber { value, radix: 2 };
                    Ok(None)
                }
                '0'..='9' | 'a'..='z' | 'A'..='Z' => {
                    value *= radix as i64;
                    let Some(digit) = c.to_digit(radix) else {
                        return Err(TokenizeError::InvalidNumber);
                    };
                    value += digit as i64;
                    self.state = InNumber { value, radix };
                    Ok(None)
                }
                _ if c.is_whitespace() => self.finalize(),
                c => {
                    // FIXME: looks messy
                    let token = self.finalize()?;
                    self.state = begin_token(c);
                    Ok(token)
                }
            },
            InOperator(ref mut op) => {
                // Pop potential unary operators early
                if matches!(op.as_str(), "+" | "-" | "(" | ")" | "*" | "/") {
                    // FIXME: looks messy
                    let token = self.finalize()?;
                    self.state = begin_token(c);
                    Ok(token)
                } else {
                    match c {
                        '0'..='9' => {
                            // FIXME: looks messy
                            let token = self.finalize()?;
                            self.state = begin_token(c);
                            Ok(token)
                        }
                        _ if c.is_whitespace() => self.finalize(),
                        _ => {
                            op.push(c);
                            Ok(None)
                        }
                    }
                }
            }
        }
    }

    pub fn finalize(&mut self) -> Result<Option<Token>, TokenizeError> {
        use TokenizerState::*;
        let token = match &mut self.state {
            Clean => None,
            InNumber { value, .. } => Some(Token::Val(*value)),
            InOperator(ref mut op) => {
                if let Some(token) = detect_operator(op) {
                    Some(token)
                } else {
                    return Err(TokenizeError::UnknownOperation(op.clone()));
                }
            }
        };
        self.state = Clean;
        Ok(token)
    }
}

fn begin_token(c: char) -> TokenizerState {
    match c {
        // 0b = binary, 0 = oct, 0x = hex
        '0' => TokenizerState::InNumber { value: 0, radix: 8 },
        '1'..='9' => TokenizerState::InNumber {
            value: c as i64 - '0' as i64,
            radix: 10,
        },
        // Ignore whitespace
        _ if c.is_whitespace() => TokenizerState::Clean,
        _ => TokenizerState::InOperator(c.to_compact_string()),
    }
}

// TODO: handle ambiguous operators
fn detect_operator(op: &mut CompactString) -> Option<Token> {
    match op.as_str() {
        "+" => Some(Token::Op(Operator::Add)),
        "-" => Some(Token::Op(Operator::Sub)),
        "/" => Some(Token::Op(Operator::Div)),
        "(" => Some(Token::ParenOpen),
        ")" => Some(Token::ParenClose),
        // TODO: Rework
        "*" => Some(Token::Op(Operator::Mul)),
        "**" => Some(Token::Op(Operator::Pow)),
        _ if op.starts_with("*") => Some(Token::Op(Operator::Mul)),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Val(Value),
    Op(Operator),
    ParenOpen,
    ParenClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

pub type Value = i64;

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
    fn test_sub_negative() {
        let result = tokenize("-12--34");
        assert_eq!(
            result,
            Ok(vec![
                Token::Op(Operator::Sub),
                Token::Val(12),
                Token::Op(Operator::Sub),
                Token::Op(Operator::Sub),
                Token::Val(34)
            ])
        );
    }
}
