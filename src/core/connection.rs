// =============================================================================
// Fichier : connection.rs
// Rôle    : Trait d'abstraction pour les connexions (Serial / SSH)
//
// Principe SOLID :
//   - Le core ne dépend d'aucun toolkit UI (pas de glib/gtk ici).
//   - Le pont UI↔core se fait dans window.rs via async_channel.
// =============================================================================

use anyhow::Result;
use async_trait::async_trait;

/// Type de connexion supporté.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Serial,
    Ssh,
}

/// État de la connexion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Déconnecté"),
            Self::Connecting => write!(f, "Connexion..."),
            Self::Connected => write!(f, "Connecté"),
            Self::Error => write!(f, "Erreur"),
        }
    }
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serial => write!(f, "Série"),
            Self::Ssh => write!(f, "SSH"),
        }
    }
}

/// Événements envoyés par la connexion vers l'UI.
///
/// SOLID : ce type n'a aucune dépendance vers GTK/glib.
#[derive(Debug)]
pub enum ConnectionEvent {
    /// Connexion établie avec succès.
    Connected {
        conn_type: ConnectionType,
        description: String,
    },
    /// Données reçues du périphérique distant.
    DataReceived(Vec<u8>),
    /// Connexion fermée proprement.
    Disconnected,
    /// Erreur non-récupérable (affichée dans le terminal).
    Error(String),
    /// Vérification de clé d'hôte SSH requise.
    ///
    /// `is_key_changed = true` indique une clé DIFFÉRENTE de celle en
    /// `known_hosts` → risque potentiel MITM. L'UI doit avertir fortement.
    /// L'UI envoie `true` (accepter) ou `false` (refuser) via `decision_tx`.
    HostKeyUnknown {
        host: String,
        key_type: String,
        fingerprint: String,
        /// `true` = clé connue MAIS différente (possible MITM).
        /// `false` = hôte inconnu (première connexion).
        is_key_changed: bool,
        decision_tx: tokio::sync::oneshot::Sender<bool>,
    },
}

/// Commandes envoyées par l'UI vers la connexion.
#[derive(Debug)]
pub enum ConnectionCommand {
    SendData(Vec<u8>),
    Disconnect,
}

/// Trait unifié pour toutes les connexions.
///
/// Permet de manipuler les connexions série et SSH de manière polymorphique.
/// SOLID : aucune dépendance UI dans ce trait.
#[async_trait]
pub trait Connection: Send {
    /// Injecte le canal d'événements **avant** `connect()`.
    ///
    /// Implémentation par défaut : no-op (connexion série l'ignore).
    /// `SshManager` l'override pour transmettre le canal à son handler de
    /// vérification de clé d'hôte.
    fn init_event_sender(&mut self, _tx: async_channel::Sender<ConnectionEvent>) {}

    /// Établit la connexion.
    async fn connect(&mut self) -> Result<()>;

    /// Ferme proprement la connexion.
    async fn disconnect(&mut self) -> Result<()>;

    /// Envoie des données brutes.
    async fn send(&mut self, data: &[u8]) -> Result<usize>;

    /// Lit les données disponibles (non-bloquant).
    /// Retourne les octets lus, ou un vecteur vide si rien n'est disponible.
    async fn read(&mut self) -> Result<Vec<u8>>;

    /// Retourne l'état courant de la connexion.
    fn state(&self) -> ConnectionState;

    /// Retourne le type de connexion.
    fn connection_type(&self) -> ConnectionType;

    /// Retourne une description de la connexion (ex: "COM3 @ 115200" ou "user@host:22").
    fn description(&self) -> String;

    /// Retourne le nombre d'octets envoyés depuis la connexion.
    fn bytes_sent(&self) -> u64;

    /// Retourne le nombre d'octets reçus depuis la connexion.
    fn bytes_received(&self) -> u64;
}

/// Lance une tâche asynchrone pour gérer la connexion.
///
/// # Architecture
/// - Entrée (UI → core) : `tokio::sync::mpsc::Sender<ConnectionCommand>`
/// - Sortie (core → UI) : `async_channel::Receiver<ConnectionEvent>`
///
/// Le core ne dépend d'aucun toolkit UI. Le pont vers `GLib` est dans window.rs.
pub fn spawn_connection_actor(
    mut connection: Box<dyn Connection>,
) -> (
    tokio::sync::mpsc::Sender<ConnectionCommand>,
    async_channel::Receiver<ConnectionEvent>,
) {
    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel::<ConnectionCommand>(32);
    // bounded(128) : backpressure si l'UI consomme trop lentement
    let (event_tx, event_rx) = async_channel::bounded::<ConnectionEvent>(128);

    // Injecter le canal avant connect() — utilisé par SshManager pour la
    // vérification interactive des clés d'hôte (SOLID : core sans dépendance GTK).
    connection.init_event_sender(event_tx.clone());

    tokio::spawn(async move {
        // ── Phase 1 : Connexion ────────────────────────────────────────────────
        // La connexion se fait dans la tâche tokio, libérant le thread GTK.
        // Pour SSH, cela permet à check_server_key d'attendre la réponse de
        // l'UI pendant que le timer GLib traite les ConnectionEvent::HostKeyUnknown.
        match connection.connect().await {
            Ok(()) => {
                let _ = event_tx
                    .send(ConnectionEvent::Connected {
                        conn_type: connection.connection_type(),
                        description: connection.description(),
                    })
                    .await;
            }
            Err(e) => {
                let _ = event_tx
                    .send(ConnectionEvent::Error(e.to_string()))
                    .await;
                return; // N'entre pas dans la boucle I/O
            }
        }

        // ── Phase 2 : Boucle I/O ──────────────────────────────────────────────
        loop {
            tokio::select! {
                biased; // prioritise les commandes UI sur la lecture

                // Commandes depuis l'UI
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(ConnectionCommand::SendData(data)) => {
                            if let Err(e) = connection.send(&data).await {
                                let _ = connection.disconnect().await;
                                let _ = event_tx.send(ConnectionEvent::Error(e.to_string())).await;
                                break;
                            }
                        }
                        Some(ConnectionCommand::Disconnect) | None => {
                            // Déconnexion propre demandée ou channel fermé
                            let _ = connection.disconnect().await;
                            let _ = event_tx.send(ConnectionEvent::Disconnected).await;
                            break;
                        }
                    }
                }

                // Lecture depuis la connexion
                read_result = connection.read() => {
                    match read_result {
                        Ok(data) if !data.is_empty() => {
                            if event_tx.send(ConnectionEvent::DataReceived(data)).await.is_err() {
                                // L'UI ne consomme plus → on arrête
                                let _ = connection.disconnect().await;
                                break;
                            }
                        }
                        Ok(_) => {
                            // Pas de données ; vérifier déconnexion spontanée
                            let s = connection.state();
                            if s == ConnectionState::Disconnected || s == ConnectionState::Error {
                                // Fermer proprement (ex: SSH envoie un message de fin)
                                let _ = connection.disconnect().await;
                                let _ = event_tx.send(ConnectionEvent::Disconnected).await;
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = connection.disconnect().await;
                            let _ = event_tx.send(ConnectionEvent::Error(e.to_string())).await;
                            break;
                        }
                    }
                }
            }
        }

        log::info!(
            "Connexion terminée — envoyés: {} octets, reçus: {} octets",
            connection.bytes_sent(),
            connection.bytes_received()
        );
        log::debug!("Acteur de connexion arrêté proprement.");
    });

    (cmd_tx, event_rx)
}
