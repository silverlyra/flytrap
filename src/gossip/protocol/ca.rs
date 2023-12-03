use std::net::{IpAddr, Ipv6Addr};

use base64::{engine::general_purpose::STANDARD as base64, Engine as _};
use rcgen::{
    date_time_ymd, BasicConstraints, Certificate, CertificateParams, DnType, DnValue,
    ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose, RcgenError, SanType, SerialNumber,
    SignatureAlgorithm, PKCS_ED25519,
};
use time::{Duration, OffsetDateTime, Time};

use crate::MachineId;

/// Issues [TLS certificates][rustls::Certificate] for QUIC cluster nodes.
pub struct CertificateAuthority(Certificate);

impl CertificateAuthority {
    fn new(params: CertificateParams) -> Self {
        Certificate::from_params(params)
            .map(Self)
            .expect("error creating cluster CA")
    }

    /// Create a [`CertificateAuthority`] by reading a base64-encoded [DER][]
    /// private key from the `$FLYTRAP_GOSSIP_KEY` environment variable.
    ///
    /// ```sh
    /// fly secrets set FLYTRAP_GOSSIP_KEY="$(openssl genpkey -algorithm ed25519 -outform DER | base64)"
    /// ```
    ///
    /// [DER]: https://en.wikipedia.org/wiki/X.690#DER_encoding
    #[cfg(feature = "environment")]
    #[cfg_attr(docsrs, doc(cfg(feature = "environment")))]
    pub fn current() -> Option<Self> {
        use std::env;

        let Ok(key) = env::var("FLYTRAP_GOSSIP_KEY") else {
            return None;
        };
        let Ok(key) = base64.decode(key) else {
            return None;
        };

        Self::from_key_der(key).ok()
    }

    fn from_key(key: KeyPair) -> Self {
        let mut params = Self::authority_params(key.algorithm());
        params.key_pair = Some(key);

        Self::new(params)
    }

    /// Create a [`CertificateAuthority`] from a private key in
    /// [DER][] encoding.
    ///
    /// [DER]: https://en.wikipedia.org/wiki/X.690#DER_encoding
    pub fn from_key_der(key: impl AsRef<[u8]>) -> Result<Self, RcgenError> {
        let key = KeyPair::from_der(key.as_ref())?;
        Ok(Self::from_key(key))
    }

    /// Create a [`CertificateAuthority`] from a private key in
    /// [PEM][] format.
    ///
    /// [PEM]: https://en.wikipedia.org/wiki/Privacy-Enhanced_Mail#Format
    pub fn from_key_pem(key: impl AsRef<str>) -> Result<Self, RcgenError> {
        let key = KeyPair::from_pem(key.as_ref())?;
        Ok(Self::from_key(key))
    }

    /// Create a [`CertificateAuthority`] with a newly-generated
    /// Ed25519 private key.
    pub fn ephemeral() -> Self {
        Self::new(Self::authority_params(&PKCS_ED25519))
    }

    /// Serialize the [certificate][rustls::Certificate] used to sign
    /// certificates issued by this CA.
    pub fn certificate(&self) -> rustls::Certificate {
        self.0
            .serialize_der()
            .map(rustls::Certificate)
            .expect("failed to serialize CA certificate")
    }

    /// Generate a [private key][rustls::PrivateKey] and
    /// [certificate][rustls::Certificate] for a machine.
    pub fn certify(
        &self,
        app: impl AsRef<str>,
        id: MachineId,
        ip: Ipv6Addr,
    ) -> (rustls::Certificate, rustls::PrivateKey) {
        let params = Self::node_params(self.0.get_params().alg, app.as_ref(), id, ip);
        let certificate =
            Certificate::from_params(params).expect("error creating node certificate");

        let key = certificate.serialize_private_key_der();
        let certificate = certificate
            .serialize_der()
            .expect("failed to serialize CA certificate");

        (rustls::Certificate(certificate), rustls::PrivateKey(key))
    }

    fn authority_params(alg: &'static SignatureAlgorithm) -> CertificateParams {
        let mut params = CertificateParams::default();

        params
            .distinguished_name
            .push(DnType::CommonName, "flytrap gossip CA");

        params.alg = alg;

        params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        params.key_usages.push(KeyUsagePurpose::DigitalSignature);
        params.key_usages.push(KeyUsagePurpose::KeyCertSign);
        params.key_usages.push(KeyUsagePurpose::CrlSign);

        params.not_before = date_time_ymd(2020, 1, 1);
        params.not_after = date_time_ymd(2030, 1, 1);

        params.serial_number = Some(1.into());

        params
    }

    fn node_params(
        alg: &'static SignatureAlgorithm,
        app: &str,
        id: MachineId,
        ip: Ipv6Addr,
    ) -> CertificateParams {
        let mut params = CertificateParams::default();

        params
            .distinguished_name
            .push(DnType::CommonName, DnValue::PrintableString(id.to_string()));
        params
            .subject_alt_names
            .push(SanType::DnsName(format!("{id}.vm.{app}.internal")));
        params
            .subject_alt_names
            .push(SanType::IpAddress(IpAddr::V6(ip)));

        params.alg = alg;

        params.use_authority_key_identifier_extension = true;
        params.key_usages.push(KeyUsagePurpose::DigitalSignature);
        params
            .extended_key_usages
            .push(ExtendedKeyUsagePurpose::ClientAuth);
        params
            .extended_key_usages
            .push(ExtendedKeyUsagePurpose::ServerAuth);

        let now = OffsetDateTime::now_utc();
        params.not_before = now.replace_time(Time::MIDNIGHT);
        params.not_after = now.saturating_add(Duration::days(90)); // TODO: handle expiration

        params.serial_number = Some(Self::serial_number(id, now));

        params
    }

    fn serial_number(id: MachineId, now: OffsetDateTime) -> SerialNumber {
        let mut bytes: Vec<u8> = Vec::with_capacity(2 * std::mem::size_of::<u64>());
        bytes.extend(id.value().to_be_bytes());
        bytes.extend(now.unix_timestamp().to_be_bytes());

        bytes.into()
    }
}
