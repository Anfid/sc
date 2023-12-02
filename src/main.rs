use anyhow::{bail, Error};

mod calculator;
mod tokenizer;

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let mut tokenizer = tokenizer::Tokenizer::default();
    let mut calculator = calculator::Calculator::default();

    for arg in std::env::args_os().skip(1) {
        let Some(utf8_arg) = arg.to_str() else {
            bail!("Arguments contain invalid UTF-8 string");
        };

        for char in utf8_arg.chars().chain(std::iter::once(' ')) {
            tokenizer
                .update(char)?
                .map(|t| calculator.handle_token(t))
                .transpose()?;
        }
    }

    tokenizer.finalize()?.map(|t| calculator.handle_token(t));
    let result = calculator.finalize()?;
    println!("{:?}", result);

    Ok(())
}
