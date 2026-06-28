mod ast;
mod checker;
mod lexer;
mod lsp;
mod parser;

#[tokio::main]
async fn main() {
    lsp::start_server().await;
}
