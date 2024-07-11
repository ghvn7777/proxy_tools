use anyhow::{Context, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use tracing::{debug, trace};

use crate::{read_exact, util::socks5::consts, AuthInfo, AuthType, VpnError};

pub struct Socks5Stream<S> {
    stream: S,
}

impl<S> Socks5Stream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    /// Read socks5 client req and connect our server
    pub async fn request(&mut self, _dns_resolve: bool) -> Result<()> {
        // todo:
        // 1. resolve dns ?
        // 2. get request address and port
        // 3. send that's info to server
        // 4. return client stream ?

        Ok(())
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
            return Err(VpnError::UnsupportedSocksVersion(version));
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

                return Err(VpnError::AuthMethodUnacceptable(client_methods));
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
            return Err(VpnError::AuthenticationFailed(format!(
                "Username malformed ({} chars)",
                user_len
            )));
        }

        let username = read_exact!(self.stream, vec![0u8; user_len as usize])
            .context("Can't get username.")?;
        debug!("username bytes: {:?}", username);

        let [pass_len] = read_exact!(self.stream, [0u8; 1]).context("Can't read pass len")?;
        debug!("Auth: [pass len: {}]", pass_len);

        if pass_len < 1 {
            return Err(VpnError::AuthenticationFailed(format!(
                "Password malformed ({} chars)",
                pass_len
            )));
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

            return Err(VpnError::AuthenticationRejected(format!(
                "Authentication, rejected({}, {})",
                credentials.username, credentials.password
            )));
        }
        // only the password way expect to write a response at this moment
        self.stream
            .write_all(&[1, consts::SOCKS5_REPLY_SUCCEEDED])
            .await
            .context("Can't reply auth success")?;
        Ok(())
    }
}
