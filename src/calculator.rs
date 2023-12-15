use crate::tokenizer::{Operator, Token, Value};
use thiserror::Error;

use CalculatorState::*;

#[derive(Debug, Default)]
enum CalculatorState {
    #[default]
    Empty,
    Neg,
    Value(Value),
}

#[derive(Debug, Default)]
pub struct Calculator {
    state: CalculatorState,
    pending: Vec<Action>,
}

impl Calculator {
    pub fn handle_token(&mut self, token: Token) -> Result<(), CalculatorError> {
        use Token::*;

        match (&self.state, token) {
            (Empty, Val(v)) => self.state = Value(v),
            (Neg, Val(v)) => self.state = Value(-v),
            // Negative sign
            (Empty, Op(Operator::Sub)) => self.state = Neg,
            // Double negative sign, cancel each other out
            (Neg, Op(Operator::Sub)) => self.state = Empty,
            // Positive sign, do nothing
            (Empty | Neg, Op(Operator::Add)) => {}
            (Empty | Neg, Op(_) | ParenClose) => return Err(CalculatorError::NumberExpected),
            (Empty, ParenOpen) => self.pending.push(Action::Parentheses(false)),
            (Neg, ParenOpen) => {
                self.pending.push(Action::Parentheses(true));
                self.state = Empty
            }
            (Value(_), Val(_)) => return Err(CalculatorError::OperationExpected),
            (Value(v), Op(op)) => {
                self.prioritized_execute(Operation { l: *v, op });
                self.state = Empty;
            }
            (Value(v), ParenOpen) => {
                self.pending.push(Action::Operation(Operation {
                    l: *v,
                    op: Operator::Mul,
                }));
                self.pending.push(Action::Parentheses(false));
                self.state = Empty;
            }
            (Value(_), ParenClose) => self.finalize_expr()?,
        }

        Ok(())
    }

    fn prioritized_execute(&mut self, mut new: Operation) {
        while let Some(pending) = self.pending.pop() {
            match pending {
                Action::Operation(op) if op.priority() >= new.priority() => {
                    new.l = op.execute(new.l)
                }
                _ => {
                    self.pending.push(pending);
                    break;
                }
            }
        }
        self.pending.push(Action::Operation(new));
    }

    fn finalize_expr(&mut self) -> Result<(), CalculatorError> {
        match self.state {
            Empty | Neg => Err(CalculatorError::NumberExpected),
            Value(mut v) => {
                while let Some(pending) = self.pending.pop() {
                    match pending {
                        Action::Parentheses(is_negative) => {
                            v = if is_negative { -v } else { v };
                            break;
                        }
                        Action::Operation(op) => v = op.execute(v),
                    }
                }
                self.state = Value(v);
                Ok(())
            }
        }
    }

    pub fn finalize(&mut self) -> Result<Value, CalculatorError> {
        self.finalize_expr()?;
        let result = match self.state {
            Empty | Neg => Err(CalculatorError::NumberExpected),
            Value(v) => Ok(v),
        };
        self.state = Empty;

        if !self.pending.is_empty() {
            return Err(CalculatorError::UnmatchedParen);
        }

        result
    }
}

#[derive(Debug)]
enum Action {
    Parentheses(bool),
    Operation(Operation),
}

#[derive(Debug)]
struct Operation {
    l: Value,
    op: Operator,
}

impl Operation {
    fn execute(self, r: Value) -> Value {
        match self.op {
            Operator::Add => self.l + r,
            Operator::Sub => self.l - r,
            Operator::Mul => self.l * r,
            // TODO: Sane div/0 handling, return NaN
            Operator::Div => self.l.checked_div(r).unwrap_or(0),
            // TODO: Validate POW number
            Operator::Pow => self.l.pow(r as u32),
        }
    }

    fn priority(&self) -> u8 {
        match self.op {
            Operator::Add | Operator::Sub => 10,
            Operator::Mul | Operator::Div => 20,
            Operator::Pow => 30,
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum CalculatorError {
    #[error("Number expected")]
    NumberExpected,
    #[error("Operation expected")]
    OperationExpected,
    #[error("Unmatched parentheses")]
    UnmatchedParen,
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADD: Token = Token::Op(Operator::Add);
    const SUB: Token = Token::Op(Operator::Sub);
    const MUL: Token = Token::Op(Operator::Mul);
    const OP: Token = Token::ParenOpen;
    const CL: Token = Token::ParenClose;

    fn calculate(tokens: Vec<Token>) -> Result<Value, CalculatorError> {
        let mut calculator = Calculator::default();

        for t in tokens {
            calculator.handle_token(t)?;
        }
        calculator.finalize()
    }

    #[test]
    fn test_negative_braces() {
        // 2 * -(2 + 2)
        let res = calculate(vec![2.into(), MUL, SUB, OP, 2.into(), ADD, 2.into(), CL]);
        assert_eq!(res, Ok(-8));
    }
}
