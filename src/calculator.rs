use crate::tokenizer::{Operation, Token, Value};
use thiserror::Error;

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
        use CalculatorState::*;
        use Token::*;

        match (&self.state, token) {
            (Empty, Val(v)) => self.state = Value(v),
            (Neg, Val(v)) => self.state = Value(-v),
            // Negative sign
            (Empty, Op(Operation::Sub)) => self.state = Neg,
            // Double negative sign, cancel each other out
            (Neg, Op(Operation::Sub)) => self.state = Empty,
            // Positive sign, do nothing
            (Empty | Neg, Op(Operation::Add)) => {}
            (Empty | Neg, Op(_)) => return Err(CalculatorError::NumberExpected),
            (Value(_), Val(_)) => return Err(CalculatorError::OperationExpected),
            (Value(v), Op(op)) => {
                self.prioritized_execute(Action { l: *v, op });
                self.state = Empty;
            }
        }

        Ok(())
    }

    fn prioritized_execute(&mut self, mut new: Action) {
        while self
            .pending
            .last()
            .map(|pending| pending.priority() >= new.priority())
            .unwrap_or(false)
        {
            new.l = self.pending.pop().unwrap().execute(new.l);
        }
        self.pending.push(new);
    }

    pub fn finalize(&mut self) -> Result<Value, CalculatorError> {
        let result = match self.state {
            CalculatorState::Empty => Err(CalculatorError::NumberExpected),
            CalculatorState::Neg => Err(CalculatorError::NumberExpected),
            CalculatorState::Value(mut v) => {
                while let Some(pending) = self.pending.pop() {
                    v = pending.execute(v);
                }
                Ok(v)
            }
        };
        self.state = CalculatorState::Empty;
        self.pending.clear();
        result
    }
}

#[derive(Debug)]
struct Action {
    l: Value,
    op: Operation,
}

impl Action {
    fn execute(self, r: Value) -> Value {
        match self.op {
            Operation::Add => self.l + r,
            Operation::Sub => self.l - r,
            Operation::Mul => self.l * r,
            Operation::Div => self.l / r,
        }
    }

    fn priority(&self) -> u8 {
        match self.op {
            Operation::Add | Operation::Sub => 10,
            Operation::Mul | Operation::Div => 20,
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum CalculatorError {
    #[error("Number expected")]
    NumberExpected,
    #[error("Operation expected")]
    OperationExpected,
}
