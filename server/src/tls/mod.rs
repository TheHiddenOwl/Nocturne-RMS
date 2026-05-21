use rcgen::{CertificateParams, DistinguishedName, KeyPair, SerialNumber, Ia5String};
use std::fs;
use std::path::Path;
use anyhow::Result;

pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

pub fn load_or_generate_certs(cert_path: &Path, key_path: &Path) -> Result<()> {
    if cert_path.exists() && key_path.exists() {
        log::info!("TLS certificates found at {:?} and {:?}", cert_path, key_path);
        return Ok(());
    }

    log::info!("Generating self-signed TLS certificates...");

    if let Some(parent) = cert_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut params = CertificateParams::default();
    params.distinguished_name = DistinguishedName::new();
    params.distinguished_name.push(rcgen::DnType::CommonName, "SpaceStationControl");
    params.subject_alt_names = vec![rcgen::SanType::DnsName(Ia5String::try_from("localhost".to_string()).unwrap())];
    params.serial_number = Some(SerialNumber::from_slice(&[1, 2, 3, 4]));

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    fs::write(cert_path, cert.pem())?;
    fs::write(key_path, key_pair.serialize_pem())?;

    log::info!("TLS certificates generated successfully.");
    Ok(())
}
