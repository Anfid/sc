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
            Clean => match c {
                // 0b = binary, 0 = oct, 0x = hex
                '0' => {
                    self.state = InNumber { value: 0, radix: 8 };
                    Ok(None)
                }
                '1'..='9' => {
                    self.state = InNumber {
                        value: c as i64 - '0' as i64,
                        radix: 10,
                    };
                    Ok(None)
                }
                '+' | '-' | '*' | '/' => {
                    self.state = InOperator(c.to_compact_string());
                    Ok(None)
                }
                // Ignore whitespace
                _ if c.is_whitespace() => Ok(None),
                // Unknown token
                _ => return Err(TokenizeError::UnexpectedToken),
            },
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
                    self.update(c)?;
                    Ok(token)
                }
            },
            InOperator(ref mut op) => {
                // Pop potential unary operators early
                if matches!(op.as_str(), "+" | "-") {
                    // FIXME: looks messy
                    let token = self.finalize()?;
                    self.update(c)?;
                    Ok(token)
                } else {
                    match c {
                        '0'..='9' => {
                            // FIXME: looks messy
                            let token = self.finalize()?;
                            self.update(c)?;
                            Ok(token)
                        }
                        '+' | '-' | '*' | '/' => {
                            op.push(c);
                            Ok(None)
                        }
                        _ if c.is_whitespace() => self.finalize(),
                        _ => Err(TokenizeError::UnexpectedToken),
                    }
                }
            }
        }
    }

    pub fn finalize(&mut self) -> Result<Option<Token>, TokenizeError> {
        use TokenizerState::*;
        let token = match &self.state {
            Clean => None,
            InNumber { value, .. } => Some(Token::Val(*value)),
            InOperator(op) => match op.as_str() {
                "+" => Some(Token::Op(Operation::Add)),
                "-" => Some(Token::Op(Operation::Sub)),
                "*" => Some(Token::Op(Operation::Mul)),
                "/" => Some(Token::Op(Operation::Div)),
                _ => return Err(TokenizeError::UnknownOperation(op.clone())),
            },
        };
        self.state = Clean;
        Ok(token)
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TokenizeError {
    #[error("Unexpected token")]
    UnexpectedToken,
    #[error("Invalid number")]
    InvalidNumber,
    #[error("Unknown operation: {0}")]
    UnknownOperation(CompactString),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Val(Value),
    Op(Operation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Add,
    Sub,
    Mul,
    Div,
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
                Token::Op(Operation::Sub),
                Token::Val(12),
                Token::Op(Operation::Sub),
                Token::Op(Operation::Sub),
                Token::Val(34)
            ])
        );
    }
}
