use std::fmt;

use clap::{
    builder::{
        styling::{AnsiColor, Effects},
        Styles,
    },
    Parser,
};

use crate::socks5::command::consts;

/// # How to use it:
/// Assume server url is 192.168.1.3：9527
/// Listen on 9020 port, authentication-free:
/// cargo run --bin client -- --server-url 192.168.1.3：9527 no-auth
///
/// Listen on 9020 port, authentication-free with chacha20 encryption:
/// cargo run --bin client -- --server-url 192.168.1.3：9527 --crypto-file /path/to/your/crypto/file no-auth
///
/// Listen on 9030 port, with basic username/password requirement:
/// cargo run --bin client -- --server-url 192.168.1.3：9527 --port 9030 auth -u admin -p password
#[derive(Debug, Parser)]
#[command(
    version,
    styles = Styles::styled()
        .header(AnsiColor::Green.on_default() | Effects::BOLD)
        .usage(AnsiColor::Green.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Cyan.on_default()))
]
pub struct ClientConfig {
    #[command(subcommand)]
    pub auth_type: AuthType,

    #[arg(long, default_value_t = 9020, help = "Socket Listener port")]
    pub port: u16,

    #[arg(
        long,
        default_value_t = false,
        help = "Resolve DNS with local DNS server [default: false]"
    )]
    pub dns_resolve: bool,

    #[arg(long, help = "Server url <IP:PORT>")]
    pub server_url: String,

    #[arg(long, help = "Tunnel count")]
    pub tunnel_cnt: Option<u32>,

    #[arg(long, help = "Path to the Frame crypto file")]
    pub crypt_file: Option<String>,
}

#[derive(Debug, Parser)]
pub enum AuthType {
    #[command(about = "No auth")]
    NoAuth,
    #[command(about = "Auth with username and password")]
    Auth(AuthInfo),
}

#[derive(Debug, Parser, PartialEq, Eq)]
pub struct AuthInfo {
    #[arg(short, long, help = "Auth username")]
    pub username: String,

    #[arg(short, long, help = "Auth password")]
    pub password: String,
}

impl AuthType {
    #[inline]
    #[rustfmt::skip]
    pub fn as_u8(&self) -> u8 {
        match self {
            AuthType::NoAuth => consts::SOCKS5_AUTH_METHOD_NONE,
            AuthType::Auth(_) =>
                consts::SOCKS5_AUTH_METHOD_PASSWORD
        }
    }

    #[inline]
    pub fn from_u8(code: u8) -> Option<AuthType> {
        match code {
            consts::SOCKS5_AUTH_METHOD_NONE => Some(AuthType::NoAuth),
            consts::SOCKS5_AUTH_METHOD_PASSWORD => Some(AuthType::Auth(AuthInfo {
                username: "admin".to_string(),
                password: "password".to_string(),
            })),
            _ => None,
        }
    }
}

impl fmt::Display for AuthType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            AuthType::NoAuth => f.write_str("AuthType::NoAuth"),
            AuthType::Auth(_) => f.write_str("AuthType::Auth"),
        }
    }
}
