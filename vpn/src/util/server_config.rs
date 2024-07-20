use clap::{
    builder::{
        styling::{AnsiColor, Effects},
        Styles,
    },
    Parser,
};

/// # How to use it:
///
/// Listen on 9527 port, no encryption:
/// cargo run --bin server
///
/// Listen on 9040 port, chacha20 encryption:
/// cargo run --bin server -- --port 9040 --crypto-file /path/to/your/crypto/file
#[derive(Debug, Parser)]
#[command(
    version,
    styles = Styles::styled()
        .header(AnsiColor::Green.on_default() | Effects::BOLD)
        .usage(AnsiColor::Green.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Cyan.on_default()))
]
pub struct ServerConfig {
    #[arg(long, default_value_t = 9527, help = "Socket Listener port")]
    pub port: u16,

    #[arg(long, help = "Path to the Frame crypto file")]
    pub crypt_file: Option<String>,
}
