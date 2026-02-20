// =============================================================================
// Fichier : ssh_manager.rs
// Rôle    : Gestionnaire de connexion SSH basé sur le trait Connection
//
// Architecture :
//  - Utilise `russh` directement (pas de wrapper) pour un contrôle maximal.
//  - `SshClientHandler` implémente `russh::client::Handler` avec une
//    vérification interactive des clés d'hôte via `async_channel`.
//  - Flow de vérification de clé (TOFU + known_hosts) :
//      1. Cherche la clé dans ~/.ssh/known_hosts.
//      2. Si connue → accepte silencieusement.
//      3. Si clé changée → alerte MITM envoyée à l'UI.
//      4. Si inconnue → demande confirmation à l'UI.
//      5. Si acceptée → enregistre dans ~/.ssh/known_hosts.
//  - Ouvre une session PTY (xterm-256color) + shell interactif.
//
// Sécurité :
//  - Aucun `unwrap()` ni `expect()`.
//  - Toutes les erreurs remontées via `anyhow::Context`.
//  - Connexion refusée si l'utilisateur rejette la clé.
// =============================================================================

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use russh::client;
use russh::keys::{self, HashAlg, PrivateKeyWithHashAlg};
use russh::keys::known_hosts::{check_known_hosts, learn_known_hosts};
use russh::{ChannelMsg, Pty};

use super::connection::{Connection, ConnectionEvent, ConnectionState, ConnectionType};

// =============================================================================
// Configuration SSH
// =============================================================================

/// Configuration d'une connexion SSH.
#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: SshAuthMethod,
    /// Délai de connexion TCP (défaut : 10 s).
    pub connect_timeout_secs: u64,
}

/// Méthode d'authentification SSH.
#[derive(Debug, Clone)]
pub enum SshAuthMethod {
    Password(String),
    KeyFile {
        private_key_path: String,
        passphrase: Option<String>,
    },
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 22,
            username: String::new(),
            auth_method: SshAuthMethod::Password(String::new()),
            connect_timeout_secs: 10,
        }
    }
}

// =============================================================================
// Handler SSH — vérification interactive des clés d'hôte
// =============================================================================

/// Handler russh gérant la vérification des clés de serveur.
///
/// Implémente le protocole TOFU (Trust On First Use) :
/// 1. Clé connue → accept silencieux.
/// 2. Clé changée → alerte MITM envoyée à l'UI.
/// 3. Clé inconnue → demande confirmation à l'UI → enregistre si acceptée.
struct SshClientHandler {
    event_tx: async_channel::Sender<ConnectionEvent>,
    host: String,
    port: u16,
}

impl client::Handler for SshClientHandler {
    type Error = anyhow::Error;

    fn check_server_key(
        &mut self,
        server_public_key: &keys::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let key = server_public_key.clone();
        let event_tx = self.event_tx.clone();
        let host = self.host.clone();
        let port = self.port;

        async move {
            let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
            let key_type = key.algorithm().to_string();

            match check_known_hosts(&host, port, &key) {
                Ok(true) => {
                    log::info!("SSH: clé connue pour {host}:{port} ({key_type}) — approuvée");
                    Ok(true)
                }

                Err(keys::Error::KeyChanged { line }) => {
                    // Clé CHANGÉE → risque MITM.
                    log::warn!(
                        "SSH: AVERTISSEMENT MITM — clé différente ligne {line} \
                         pour {host}:{port} ! fingerprint: {fingerprint}"
                    );
                    let (decision_tx, decision_rx) = tokio::sync::oneshot::channel::<bool>();
                    let _ = event_tx
                        .send(ConnectionEvent::HostKeyUnknown {
                            host: host.clone(),
                            key_type,
                            fingerprint,
                            is_key_changed: true,
                            decision_tx,
                        })
                        .await;
                    let accepted = tokio::time::timeout(Duration::from_secs(300), decision_rx)
                        .await
                        .ok()
                        .and_then(std::result::Result::ok)
                        .unwrap_or(false);
                    if accepted {
                        if let Err(e) = learn_known_hosts(&host, port, &key) {
                            log::warn!("SSH: impossible d'enregistrer la clé : {e}");
                        }
                    }
                    Ok(accepted)
                }

                Ok(false) | Err(_) => {
                    // Hôte inconnu — première connexion.
                    log::info!("SSH: hôte inconnu {host}:{port} — demande confirmation");
                    let (decision_tx, decision_rx) = tokio::sync::oneshot::channel::<bool>();
                    let _ = event_tx
                        .send(ConnectionEvent::HostKeyUnknown {
                            host: host.clone(),
                            key_type,
                            fingerprint,
                            is_key_changed: false,
                            decision_tx,
                        })
                        .await;
                    let accepted = tokio::time::timeout(Duration::from_secs(300), decision_rx)
                        .await
                        .ok()
                        .and_then(std::result::Result::ok)
                        .unwrap_or(false);
                    if accepted {
                        if let Err(e) = learn_known_hosts(&host, port, &key) {
                            log::warn!("SSH: impossible d'enregistrer la clé dans known_hosts : {e}");
                        } else {
                            log::info!("SSH: clé de {host}:{port} ajoutée à ~/.ssh/known_hosts");
                        }
                    }
                    Ok(accepted)
                }
            }
        }
    }
}

// =============================================================================
// Gestionnaire SSH
// =============================================================================

/// Gestionnaire de connexion SSH implémentant le trait `Connection`.
pub struct SshManager {
    config: SshConfig,
    /// Handle russh (connexion TCP + protocole SSH).
    handle: Option<client::Handle<SshClientHandler>>,
    /// Canal de session SSH avec PTY + shell.
    channel: Option<russh::Channel<client::Msg>>,
    state: ConnectionState,
    bytes_sent: u64,
    bytes_received: u64,
    /// Canal d'événements injecté par `spawn_connection_actor` avant `connect()`.
    event_tx: Option<async_channel::Sender<ConnectionEvent>>,
}

impl SshManager {
    /// Crée un nouveau gestionnaire SSH avec la configuration donnée.
    pub const fn new(config: SshConfig) -> Self {
        Self {
            config,
            handle: None,
            channel: None,
            state: ConnectionState::Disconnected,
            bytes_sent: 0,
            bytes_received: 0,
            event_tx: None,
        }
    }
}

#[async_trait]
impl Connection for SshManager {
    fn init_event_sender(&mut self, tx: async_channel::Sender<ConnectionEvent>) {
        self.event_tx = Some(tx);
    }

    async fn connect(&mut self) -> Result<()> {
        if self.state == ConnectionState::Connected {
            bail!("Déjà connecté à {}:{}", self.config.host, self.config.port);
        }

        let event_tx = self
            .event_tx
            .clone()
            .context("Canal d'événements non initialisé")?;

        self.state = ConnectionState::Connecting;
        let addr = format!("{}:{}", self.config.host, self.config.port);
        log::info!("Connexion SSH vers {addr}...");

        let ssh_config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(self.config.connect_timeout_secs * 3)),
            keepalive_interval: Some(Duration::from_secs(15)),
            keepalive_max: 3,
            ..<client::Config as Default>::default()
        });

        let handler = SshClientHandler {
            event_tx,
            host: self.config.host.clone(),
            port: self.config.port,
        };

        let mut handle = match tokio::time::timeout(
            Duration::from_secs(self.config.connect_timeout_secs + 2),
            client::connect(ssh_config, addr.as_str(), handler),
        )
        .await
        {
            Ok(Ok(h)) => h,
            Ok(Err(e)) => {
                self.state = ConnectionState::Disconnected;
                return Err(e).context("Impossible d'établir la connexion SSH");
            }
            Err(_) => {
                self.state = ConnectionState::Disconnected;
                bail!("Timeout de connexion SSH vers {addr}");
            }
        };

        // Authentification
        let auth_result = match &self.config.auth_method {
            SshAuthMethod::Password(password) => handle
                .authenticate_password(&self.config.username, password)
                .await
                .context("Erreur lors de l'authentification par mot de passe")?,

            SshAuthMethod::KeyFile { private_key_path, passphrase } => {
                let key = keys::load_secret_key(private_key_path, passphrase.as_deref())
                    .context("Impossible de charger la clé privée SSH")?;
                let key_with_alg = PrivateKeyWithHashAlg::new(
                    Arc::new(key),
                    Some(HashAlg::Sha256),
                );
                handle
                    .authenticate_publickey(&self.config.username, key_with_alg)
                    .await
                    .context("Erreur lors de l'authentification par clé publique")?
            }
        };

        if !auth_result.success() {
            self.state = ConnectionState::Disconnected;
            let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "en").await;
            bail!(
                "Authentification SSH échouée pour {}@{}:{}",
                self.config.username,
                self.config.host,
                self.config.port
            );
        }

        // Session interactive avec PTY xterm-256color + shell
        let channel = match handle.channel_open_session().await {
            Ok(c) => c,
            Err(e) => {
                self.state = ConnectionState::Disconnected;
                let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "en").await;
                return Err(e).context("Impossible d'ouvrir un canal de session SSH");
            }
        };

        if let Err(e) = channel
            .request_pty(true, "xterm-256color", 220, 50, 0, 0, &[(Pty::ECHO, 1), (Pty::ICANON, 1)])
            .await
        {
            self.state = ConnectionState::Disconnected;
            let _ = channel.close().await;
            let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "en").await;
            return Err(e).context("Impossible d'obtenir un PTY SSH");
        }

        if let Err(e) = channel.request_shell(true).await {
            self.state = ConnectionState::Disconnected;
            let _ = channel.close().await;
            let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "en").await;
            return Err(e).context("Impossible de démarrer le shell SSH");
        }

        self.handle = Some(handle);
        self.channel = Some(channel);
        self.state = ConnectionState::Connected;
        self.bytes_sent = 0;
        self.bytes_received = 0;

        log::info!(
            "Connecté SSH à {}@{}:{} (PTY xterm-256color + shell)",
            self.config.username,
            self.config.host,
            self.config.port
        );
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if self.state == ConnectionState::Disconnected {
            return Ok(());
        }

        log::info!("Déconnexion SSH de {}:{}...", self.config.host, self.config.port);

        if let Some(channel) = self.channel.take() {
            let _ = channel.close().await;
        }

        if let Some(handle) = self.handle.take() {
            let _ = handle
                .disconnect(russh::Disconnect::ByApplication, "", "en")
                .await;
        }

        self.state = ConnectionState::Disconnected;
        log::info!(
            "Déconnecté SSH (envoyés: {} octets, reçus: {} octets)",
            self.bytes_sent,
            self.bytes_received
        );
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        let channel = self.channel.as_mut().context("Canal SSH non disponible")?;
        channel.data(data).await.context("Erreur d'écriture SSH")?;
        self.bytes_sent += data.len() as u64;
        Ok(data.len())
    }

    async fn read(&mut self) -> Result<Vec<u8>> {
        let channel = self.channel.as_mut().context("Canal SSH non disponible")?;

        match tokio::time::timeout(Duration::from_millis(10), channel.wait()).await {
            Ok(Some(ChannelMsg::Data { data })) => {
                let len = data.len();
                self.bytes_received += len as u64;
                Ok(data.to_vec())
            }
            Ok(Some(ChannelMsg::ExtendedData { data, .. })) => {
                // stderr du serveur — on l'affiche également
                let len = data.len();
                self.bytes_received += len as u64;
                Ok(data.to_vec())
            }
            Ok(Some(ChannelMsg::Eof | ChannelMsg::Close)) => {
                self.state = ConnectionState::Disconnected;
                log::info!("Canal SSH fermé par le serveur distant");
                Ok(Vec::new())
            }
            Ok(Some(ChannelMsg::Success | _)) => {
                // Messages de contrôle ignorés
                Ok(Vec::new())
            }
            Ok(None) => {
                self.state = ConnectionState::Disconnected;
                Ok(Vec::new())
            }
            Err(_) => {
                // Timeout normal — pas de données disponibles
                Ok(Vec::new())
            }
        }
    }

    fn state(&self) -> ConnectionState {
        self.state
    }

    fn connection_type(&self) -> ConnectionType {
        ConnectionType::Ssh
    }

    fn description(&self) -> String {
        format!(
            "{}@{}:{}",
            self.config.username, self.config.host, self.config.port
        )
    }

    fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    fn bytes_received(&self) -> u64 {
        self.bytes_received
    }
}
