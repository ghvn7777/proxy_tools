use clap::Parser;
use vpn::process_genpass;

#[derive(Debug, Parser)]
pub struct GenPassOpts {
    #[arg(long, default_value_t = true)]
    pub uppercase: bool,

    #[arg(long, default_value_t = true)]
    pub lowercase: bool,

    #[arg(long, default_value_t = true)]
    pub number: bool,

    #[arg(long, default_value_t = true)]
    pub symbol: bool,
}

impl GenPassOpts {
    async fn execute(self) -> anyhow::Result<()> {
        let ret = process_genpass(
            44, // chacha20 32 bytes key + 12 bytes nonce
            self.uppercase,
            self.lowercase,
            self.number,
            self.symbol,
        )?;
        println!("{}", ret);

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = GenPassOpts::parse();
    opts.execute().await
}
