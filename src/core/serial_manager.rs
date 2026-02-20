// =============================================================================
// Fichier : serial_manager.rs
// Rôle    : Gestionnaire de connexion série basé sur le trait Connection
// =============================================================================

use std::time::Duration;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serialport::{available_ports, DataBits, FlowControl, Parity, StopBits};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use super::connection::{Connection, ConnectionState, ConnectionType};

// =============================================================================
// Information sur un port série
// =============================================================================

/// Décrit un port série détecté sur le système.
#[derive(Debug, Clone)]
pub struct SerialPortInfo {
    pub device: String,
    pub manufacturer: String,
    pub description: String,
}

/// Liste les ports série disponibles sur le système.
pub fn list_serial_ports() -> Vec<SerialPortInfo> {
    match available_ports() {
        Ok(ports) => ports
            .into_iter()
            .map(|p| {
                let (manufacturer, description) = match &p.port_type {
                    serialport::SerialPortType::UsbPort(info) => (
                        info.manufacturer.clone().unwrap_or_default(),
                        info.product.clone().unwrap_or_default(),
                    ),
                    _ => (String::new(), String::new()),
                };
                SerialPortInfo {
                    device: p.port_name,
                    manufacturer,
                    description,
                }
            })
            .collect(),
        Err(e) => {
            log::warn!("Impossible d'énumérer les ports série : {e}");
            Vec::new()
        }
    }
}

// =============================================================================
// Gestionnaire de connexion série
// =============================================================================

/// Configuration d'une connexion série.
#[derive(Debug, Clone)]
pub struct SerialConfig {
    pub port: String,
    pub baudrate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub flow_control: FlowControl,
    pub timeout: Duration,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port: String::new(),
            baudrate: 115_200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
            timeout: Duration::from_millis(10),
        }
    }
}

impl SerialConfig {
    /// Construit la configuration à partir des paramètres utilisateur.
    pub fn from_params(
        port: &str,
        baudrate: u32,
        data_bits: u8,
        parity: &str,
        stop_bits: u8,
        flow_control: &str,
        timeout_ms: u64,
    ) -> Self {
        Self {
            port: port.to_string(),
            baudrate,
            data_bits: match data_bits {
                5 => DataBits::Five,
                6 => DataBits::Six,
                7 => DataBits::Seven,
                _ => DataBits::Eight,
            },
            parity: match parity {
                "Odd" => Parity::Odd,
                "Even" => Parity::Even,
                _ => Parity::None,
            },
            stop_bits: match stop_bits {
                2 => StopBits::Two,
                _ => StopBits::One,
            },
            flow_control: match flow_control {
                "Hardware" => FlowControl::Hardware,
                "Software" => FlowControl::Software,
                _ => FlowControl::None,
            },
            timeout: Duration::from_millis(timeout_ms),
        }
    }
}

/// Gestionnaire de connexion série implémentant le trait `Connection`.
pub struct SerialManager {
    config: SerialConfig,
    port: Option<SerialStream>,
    state: ConnectionState,
    bytes_sent: u64,
    bytes_received: u64,
}

impl SerialManager {
    /// Crée un nouveau gestionnaire avec la configuration donnée.
    pub const fn new(config: SerialConfig) -> Self {
        Self {
            config,
            port: None,
            state: ConnectionState::Disconnected,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}

#[async_trait]
impl Connection for SerialManager {
    async fn connect(&mut self) -> Result<()> {
        if self.state == ConnectionState::Connected {
            bail!("Déjà connecté à {}", self.config.port);
        }

        self.state = ConnectionState::Connecting;
        log::info!(
            "Connexion série vers {} @ {}...",
            self.config.port,
            self.config.baudrate
        );

        let port = tokio_serial::new(&self.config.port, self.config.baudrate)
            .data_bits(self.config.data_bits)
            .parity(self.config.parity)
            .stop_bits(self.config.stop_bits)
            .flow_control(self.config.flow_control)
            .timeout(self.config.timeout)
            .open_native_async()
            .with_context(|| format!("Impossible d'ouvrir le port {}", self.config.port))?;

        self.port = Some(port);
        self.state = ConnectionState::Connected;
        self.bytes_sent = 0;
        self.bytes_received = 0;
        log::info!("Connecté à {} @ {}", self.config.port, self.config.baudrate);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if self.state == ConnectionState::Disconnected {
            return Ok(());
        }

        log::info!("Déconnexion série de {}...", self.config.port);
        self.port = None; // Drop ferme le port
        self.state = ConnectionState::Disconnected;
        log::info!(
            "Déconnecté de {} (envoyés: {} octets, reçus: {} octets)",
            self.config.port,
            self.bytes_sent,
            self.bytes_received
        );
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        let port = self.port.as_mut().context("Port série non connecté")?;

        let written = port.write(data).await.context("Erreur d'écriture série")?;
        port.flush().await.context("Erreur de flush série")?;
        self.bytes_sent += written as u64;
        Ok(written)
    }

    async fn read(&mut self) -> Result<Vec<u8>> {
        let port = self.port.as_mut().context("Port série non connecté")?;

        let mut buf = vec![0u8; 4096];

        match port.read(&mut buf).await {
            Ok(0) => {
                // EOF
                self.state = ConnectionState::Disconnected;
                Ok(Vec::new())
            }
            Ok(n) => {
                buf.truncate(n);
                self.bytes_received += n as u64;
                Ok(buf)
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(Vec::new()),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(Vec::new()),
            Err(e) => {
                self.state = ConnectionState::Error;
                Err(e).context("Erreur de lecture série")
            }
        }
    }

    fn state(&self) -> ConnectionState {
        self.state
    }

    fn connection_type(&self) -> ConnectionType {
        ConnectionType::Serial
    }

    fn description(&self) -> String {
        format!("{} @ {}", self.config.port, self.config.baudrate)
    }

    fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    fn bytes_received(&self) -> u64 {
        self.bytes_received
    }
}
