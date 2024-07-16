use std::net::SocketAddr;

use super::command::{consts, Socks5Command};
use anyhow::{Context, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use tracing::{debug, trace};

use crate::{
    read_exact,
    util::{read_address, TargetAddr},
    AuthInfo, AuthType, Socks5Error, VpnError,
};

pub struct Socks5Stream<S> {
    pub stream: S,
}

impl<S> Socks5Stream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    /// Read socks5 client req and connect our server
    pub async fn request(
        &mut self,
        dns_resolve: bool,
    ) -> Result<(Socks5Command, TargetAddr), VpnError> {
        trace!("Socks5Stream: request");

        let (cmd, mut target_addr) = self.read_command().await?;
        if dns_resolve {
            target_addr = target_addr.resolve_dns().await?;
        } else {
            debug!("Domain won't be resolved because `dns_resolve`'s config has been turned off.");
        }

        Ok((cmd, target_addr))
    }

    pub async fn handshake(&mut self, auth_type: &AuthType) -> Result<(), VpnError> {
        trace!("Socks5Stream: handshake");
        let methods = self.get_methods().await?;
        self.can_accept_method(methods, auth_type).await?;
        if let AuthType::Auth(auth_info) = auth_type {
            self.authenticate(auth_info).await?;
        }
        Ok(())
    }

    /// Execute the socks5 command that the client wants.
    pub async fn execute_command(&mut self, cmd: Socks5Command) -> Result<bool, VpnError> {
        match cmd {
            Socks5Command::TCPBind => Err(Socks5Error::SocksCommandNotSupported.into()),
            Socks5Command::TCPConnect => Ok(true),
            Socks5Command::UDPAssociate => Ok(false),
        }
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, VpnError> {
        trace!("Socks5Stream: read");
        Ok(self
            .stream
            .read(buf)
            .await
            .context("Can't read from stream")?)
    }

    pub async fn write(&mut self, buf: &[u8]) -> Result<usize, VpnError> {
        trace!("Socks5Stream: write");
        Ok(self
            .stream
            .write(buf)
            .await
            .context("Can't write to stream")?)
    }

    pub async fn send_reply(&mut self, rep: u8, sock_addr: SocketAddr) -> Result<(), VpnError> {
        trace!("Socks5Stream: send_reply");
        let reply = Self::new_reply(rep, sock_addr);
        self.stream
            .write_all(&reply)
            .await
            .context("Can't send reply")?;
        self.stream
            .flush()
            .await
            .context("Can't flush the reply!")?;
        Ok(())
    }

    /// Generate reply code according to the RFC.
    fn new_reply(rep: u8, sock_addr: SocketAddr) -> Vec<u8> {
        trace!("Socks5Stream: new_reply");
        let (addr_type, mut ip_oct, mut port) = match sock_addr {
            SocketAddr::V4(sock) => (
                consts::SOCKS5_ADDR_TYPE_IPV4,
                sock.ip().octets().to_vec(),
                sock.port().to_be_bytes().to_vec(),
            ),
            SocketAddr::V6(sock) => (
                consts::SOCKS5_ADDR_TYPE_IPV6,
                sock.ip().octets().to_vec(),
                sock.port().to_be_bytes().to_vec(),
            ),
        };

        let mut reply = vec![
            consts::SOCKS5_VERSION,
            rep,       // transform the error into byte code
            0x00,      // reserved
            addr_type, // address type (ipv4, v6, domain)
        ];
        reply.append(&mut ip_oct);
        reply.append(&mut port);

        reply
    }

    /// Decide to whether or not, accept the authentication method.
    /// Don't forget that the methods list sent by the client, contains one or more methods.
    ///
    /// # Request
    /// ```text
    ///          +----+-----+-------+------+----------+----------+
    ///          |VER | CMD |  RSV  | ATYP | DST.ADDR | DST.PORT |
    ///          +----+-----+-------+------+----------+----------+
    ///          | 1  |  1  |   1   |  1   | Variable |    2     |
    ///          +----+-----+-------+------+----------+----------+
    /// ```
    ///
    /// It the request is correct, it should returns a ['SocketAddr'].
    ///
    async fn read_command(&mut self) -> Result<(Socks5Command, TargetAddr), VpnError> {
        trace!("Socks5Stream: read_command");
        let [version, cmd, rsv, address_type] =
            read_exact!(self.stream, [0u8; 4]).context("Malformed request")?;
        debug!(
            "Request: [version: {version}, command: {cmd}, rev: {rsv}, address_type: {address_type}]",
            version = version,
            cmd = cmd,
            rsv = rsv,
            address_type = address_type,
        );

        if version != consts::SOCKS5_VERSION {
            return Err(Socks5Error::UnsupportedSocksVersion(version).into());
        }

        let cmd = match Socks5Command::from_u8(cmd) {
            None => return Err(Socks5Error::SocksCommandNotSupported.into()),
            Some(cmd) => match cmd {
                Socks5Command::TCPConnect => cmd,
                Socks5Command::UDPAssociate => cmd,
                Socks5Command::TCPBind => return Err(Socks5Error::SocksCommandNotSupported.into()),
            },
        };

        // Guess address type
        let target_addr = read_address(&mut self.stream, address_type).await?;

        debug!("Request target is {}", target_addr);

        Ok((cmd, target_addr))
    }

    /// Read the authentication method provided by the client.
    /// A client send a list of methods that he supports, he could send
    ///
    ///   - 0: Non auth
    ///   - 2: Auth with username/password
    ///
    /// Altogether, then the server choose to use of of these,
    /// or deny the handshake (thus the connection).
    ///
    /// # Examples
    /// ```text
    ///                    {SOCKS Version, methods-length}
    ///     eg. (non-auth) {5, 2}
    ///     eg. (auth)     {5, 3}
    /// ```
    ///
    async fn get_methods(&mut self) -> Result<Vec<u8>, VpnError> {
        trace!("Socks5Stream: get_methods()");
        // read the first 2 bytes which contains the SOCKS version and the methods len()
        let [version, methods_len] =
            read_exact!(self.stream, [0u8; 2]).context("Can't read methods")?;
        debug!(
            "Handshake headers: [version: {version}, methods len: {len}]",
            version = version,
            len = methods_len,
        );

        if version != consts::SOCKS5_VERSION {
            return Err(Socks5Error::UnsupportedSocksVersion(version).into());
        }

        // {METHODS available from the client}
        // eg. (non-auth) {0, 1}
        // eg. (auth)     {0, 1, 2}
        let methods = read_exact!(self.stream, vec![0u8; methods_len as usize])
            .context("Can't get methods.")?;
        debug!("Methods supported sent by the client: {:?}", &methods);

        // Return methods available
        Ok(methods)
    }

    /// Decide to whether or not, accept the authentication method.
    /// Don't forget that the methods list sent by the client, contains one or more methods.
    ///
    /// # Request
    ///
    ///  Client send an array of 3 entries: [0, 1, 2]
    /// ```text
    ///                          {SOCKS Version,  Authentication chosen}
    ///     eg. (non-auth)       {5, 0}
    ///     eg. (GSSAPI)         {5, 1}
    ///     eg. (auth)           {5, 2}
    /// ```
    ///
    /// # Response
    /// ```text
    ///     eg. (accept non-auth) {5, 0x00}
    ///     eg. (non-acceptable)  {5, 0xff}
    /// ```
    ///
    async fn can_accept_method(
        &mut self,
        client_methods: Vec<u8>,
        auth_type: &AuthType,
    ) -> Result<(), VpnError> {
        let method_supported;

        if let AuthType::Auth(_auth_info) = auth_type {
            if client_methods.contains(&consts::SOCKS5_AUTH_METHOD_PASSWORD) {
                // can auth with password
                method_supported = consts::SOCKS5_AUTH_METHOD_PASSWORD;
            } else {
                // we don't allow no auth, so we deny the entry
                debug!("Don't support this auth method, reply with (0xff)");
                self.stream
                    .write_all(&[
                        consts::SOCKS5_VERSION,
                        consts::SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE,
                    ])
                    .await
                    .context("Can't reply with method not acceptable.")?;

                return Err(Socks5Error::AuthMethodUnacceptable(client_methods).into());
            }
        } else {
            method_supported = consts::SOCKS5_AUTH_METHOD_NONE;
        }

        debug!(
            "Reply with method {} ({})",
            AuthType::from_u8(method_supported).context("Method not supported")?,
            method_supported
        );
        self.stream
            .write(&[consts::SOCKS5_VERSION, method_supported])
            .await
            .context("Can't reply with method auth-none")?;
        Ok(())
    }

    /// Request:
    /// --------------------------------------------------------------------
    /// | Auth version | username len | username | password len | password |
    /// --------------------------------------------------------------------
    ///
    /// - Auth version currency only 1
    async fn read_username_password(&mut self) -> Result<AuthInfo, VpnError> {
        trace!("Socks5Stream: read_username_password");
        let [version, user_len] =
            read_exact!(self.stream, [0u8; 2]).context("Can't read user len")?;
        debug!(
            "Auth: [version: {version}, user len: {len}]",
            version = version,
            len = user_len,
        );

        if user_len < 1 {
            return Err(Socks5Error::AuthenticationFailed(format!(
                "Username malformed ({} chars)",
                user_len
            ))
            .into());
        }

        let username = read_exact!(self.stream, vec![0u8; user_len as usize])
            .context("Can't get username.")?;
        debug!("username bytes: {:?}", username);

        let [pass_len] = read_exact!(self.stream, [0u8; 1]).context("Can't read pass len")?;
        debug!("Auth: [pass len: {}]", pass_len);

        if pass_len < 1 {
            return Err(Socks5Error::AuthenticationFailed(format!(
                "Password malformed ({} chars)",
                pass_len
            ))
            .into());
        }

        let password = read_exact!(self.stream, vec![0u8; pass_len as usize])
            .context("Can't get password.")?;
        debug!("password bytes: {:?}", &password);

        let username = String::from_utf8(username).context("Failed to convert username")?;
        let password = String::from_utf8(password).context("Failed to convert password")?;

        Ok(AuthInfo { username, password })
    }

    /// Only called if socket server supports authentication via username / password
    /// Auth Response:
    /// ------------------------------
    /// | Auth version | Auth Status |
    /// ------------------------------
    ///
    /// - Auth version only 1
    /// - Auth Status:
    ///     - success: 0
    ///     - failed: 1
    async fn authenticate(&mut self, auth_info: &AuthInfo) -> Result<(), VpnError> {
        trace!("Socks5Stream: authenticate");

        let credentials = self.read_username_password().await?;
        if &credentials != auth_info {
            self.stream
                .write_all(&[1, consts::SOCKS5_REPLY_GENERAL_FAILURE])
                .await
                .context("Can't reply with auth method not acceptable.")?;

            return Err(Socks5Error::AuthenticationRejected(format!(
                "Authentication, rejected({}, {})",
                credentials.username, credentials.password
            ))
            .into());
        }
        // only the password way expect to write a response at this moment
        self.stream
            .write_all(&[1, consts::SOCKS5_REPLY_SUCCEEDED])
            .await
            .context("Can't reply auth success")?;
        Ok(())
    }
}
