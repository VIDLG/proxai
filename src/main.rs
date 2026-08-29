mod cli;

fn main() -> anyhow::Result<()> {
    proxai::ensure_rustls_crypto_provider();
    cli::main()
}
