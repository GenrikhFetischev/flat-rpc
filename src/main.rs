extern crate core;

mod parser;
mod ir;
mod codegen_rs;
mod typechecker;
mod codegen_ts;

use std::fs;
use clap::Parser;

use colored::Colorize;
use crate::codegen_rs::generate_rust_server_side_code;
use crate::codegen_ts::generate_ts_client_side_code;
use crate::parser::parse_fbs_schema;
use crate::typechecker::type_check;


#[derive(clap::ArgEnum, Debug, Clone)]
enum Side {
  Client,
  Server,
}

#[derive(clap::ArgEnum, Debug, Clone)]
enum Lang {
  Rust,
  Ts,
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
  #[clap(short, long)]
  input_file: String,
  #[clap(short, long)]
  output_file: Option<String>,
  #[clap(short, long, arg_enum)]
  side: Side,
  #[clap(short, long, arg_enum)]
  lang: Lang,
}


fn main() {
  let Args { input_file, output_file, side, lang } = Args::parse();

  let unparsed_file = match fs::read_to_string(&input_file) {
    Ok(file) => file,
    Err(e) => {
      println!(
        "{} {}",
        "💔 Can't read prisma file in path:".yellow(),
        input_file.cyan(),
      );

      panic!("{}", e)
    }
  };
  let statements = parse_fbs_schema(&unparsed_file);


  if let Some(errors) = type_check(&statements) {
    panic!("{errors}");
  }

  let generated_code = match (side, lang) {
    (Side::Server, Lang::Rust) => {
      generate_rust_server_side_code(&statements)
    }
    (Side::Client, Lang::Rust) => {
      todo!("Generate code for clients in rust isn't ready");
    }
    (Side::Client, Lang::Ts) => {
      generate_ts_client_side_code(&statements)
    }
    (Side::Server, Lang::Ts) => {
      todo!("Actually, I don't have a plan to do so, at least right now");
    }
  };


  if let Some(path) = output_file {
    fs::write(path, generated_code).expect("Can't write to output path");
  } else {
    println!("{generated_code}");
  }
}
