use std::{io, net::SocketAddr};

use rustls::{Certificate, PrivateKey};

use super::protocol::ca::CertificateAuthority;

pub struct Endpoint {
    inner: quinn::Endpoint,
}

impl Endpoint {
    pub fn new(addr: SocketAddr, cert: Certificate, key: PrivateKey) -> Result<Self, io::Error> {
        let server_config = quinn::ServerConfig::with_single_cert(vec![cert], key)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        let endpoint = quinn::Endpoint::server(server_config, addr)?;

        Ok(Self { inner: endpoint })
    }
}
