#![allow(dead_code)]
mod ast;
mod checker;
mod codegen;
mod lexer;
mod lsp;
mod parser;
mod yard;

#[tokio::main]
async fn main() {
    lsp::start_server().await;
}
