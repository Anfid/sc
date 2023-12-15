use anyhow::{bail, Error};
use std::io::BufRead;
use std::io::Write;

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

    let args = std::env::args_os().skip(1);
    let stdout = std::io::stdout();
    let lock = stdout.lock();
    let mut w = std::io::BufWriter::new(lock);

    if args.len() > 0 {
        for arg in args {
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
        writeln!(&mut w, "{}", result)?;
    } else {
        let stdin = std::io::stdin();
        let reader = std::io::BufReader::new(stdin);
        let is_interactive = atty::is(atty::Stream::Stdin);

        if is_interactive {
            write!(&mut w, ">>> ")?;
            w.flush()?;
        }

        for expr in reader.lines() {
            for char in expr?.chars() {
                tokenizer
                    .update(char)?
                    .map(|t| calculator.handle_token(t))
                    .transpose()?;
            }

            tokenizer.finalize()?.map(|t| calculator.handle_token(t));
            let result = calculator.finalize()?;

            writeln!(&mut w, "{}", result)?;
            if is_interactive {
                write!(&mut w, ">>> ")?;
                w.flush()?;
            }
        }
    }
    w.flush()?;

    Ok(())
}
