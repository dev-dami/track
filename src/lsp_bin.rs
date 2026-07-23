use track::lsp;

#[tokio::main]
async fn main() {
    lsp::start_server().await;
}
