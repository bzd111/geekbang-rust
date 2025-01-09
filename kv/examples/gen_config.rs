use std::fs;

use anyhow::Result;
use kv::{
    ClientConfig, ClientTlsConfig, GeneralConfig, LogConfig, RotationConfig, ServerConfig,
    ServerTlsConfig, StorageConfig,
};
fn main() -> Result<()> {
    // const CA_CERT: &str = include!("../fixtures/ca.cert");
    // const SERVER_CERT: &str = include!("../fixtures/server.cert");
    // const SERVER_KEY: &str = include!("../fixtures/server.key");

    const CA_CERT: &str = include_str!("../fixtures/ca.cert");
    const SERVER_CERT: &str = include_str!("../fixtures/server.cert");
    const SERVER_KEY: &str = include_str!("../fixtures/server.key");

    let general_config = GeneralConfig {
        addr: "127.0.0.1:9527".to_string(),
    };
    let server_config = ServerConfig {
        storage: StorageConfig::SledDb("/tmp/kv_server".into()),
        general: general_config.clone(),
        log: LogConfig {
            path: "/tmp/kv-log".to_string(),
            rotation: RotationConfig::Daily,
        },
        tls: ServerTlsConfig {
            cert: SERVER_CERT.to_string(),
            key: SERVER_KEY.to_string(),
            ca: None,
        },
    };

    let _ = fs::write(
        "fixtures/server.conf",
        toml::to_string_pretty(&server_config)?,
    );

    let client_config = ClientConfig {
        general: general_config,
        tls: ClientTlsConfig {
            domain: "kvserver.acme.inc".to_string(),
            identity: None,
            ca: Some(CA_CERT.to_string()),
        },
    };
    let _ = fs::write(
        "fixtures/client.conf",
        toml::to_string_pretty(&client_config)?,
    );
    Ok(())
}
