// =============================================================================
// Fichier : secrets.rs
// Rôle    : Stockage sécurisé des secrets via le trousseau système
// =============================================================================

use anyhow::{anyhow, Result};

const SERVICE_NAME: &str = "com.github.weedmanu.serial-ssh-term";

fn password_account(host: &str, port: u16, username: &str) -> String {
    format!("ssh-password:{username}@{host}:{port}")
}

fn passphrase_account(host: &str, port: u16, username: &str, key_path: &str) -> String {
    format!(
        "ssh-passphrase:{username}@{host}:{port}:{}",
        key_path.trim()
    )
}

fn save_secret(account: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Ok(());
    }

    let entry = keyring::Entry::new(SERVICE_NAME, account)
        .map_err(|e| anyhow!("Impossible d'initialiser le keyring: {e}"))?;
    entry
        .set_password(value)
        .map_err(|e| anyhow!("Impossible d'écrire le secret dans le keyring: {e}"))
}

fn delete_secret(account: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, account)
        .map_err(|e| anyhow!("Impossible d'initialiser le keyring: {e}"))?;

    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow!("Impossible de supprimer le secret du keyring: {e}")),
    }
}

fn load_secret(account: &str) -> Option<String> {
    let entry = match keyring::Entry::new(SERVICE_NAME, account) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("Keyring non disponible: {e}");
            return None;
        }
    };

    match entry.get_password() {
        Ok(value) => Some(value),
        Err(keyring::Error::NoEntry) => None,
        Err(e) => {
            log::warn!("Lecture keyring impossible: {e}");
            None
        }
    }
}

pub fn save_ssh_password(host: &str, port: u16, username: &str, password: &str) -> Result<()> {
    save_secret(&password_account(host, port, username), password)
}

pub fn load_ssh_password(host: &str, port: u16, username: &str) -> Option<String> {
    load_secret(&password_account(host, port, username))
}

pub fn delete_ssh_password(host: &str, port: u16, username: &str) -> Result<()> {
    delete_secret(&password_account(host, port, username))
}

pub fn save_ssh_key_passphrase(
    host: &str,
    port: u16,
    username: &str,
    key_path: &str,
    passphrase: &str,
) -> Result<()> {
    if key_path.trim().is_empty() {
        return Ok(());
    }

    save_secret(
        &passphrase_account(host, port, username, key_path),
        passphrase,
    )
}

pub fn load_ssh_key_passphrase(
    host: &str,
    port: u16,
    username: &str,
    key_path: &str,
) -> Option<String> {
    if key_path.trim().is_empty() {
        return None;
    }

    load_secret(&passphrase_account(host, port, username, key_path))
}

pub fn delete_ssh_key_passphrase(
    host: &str,
    port: u16,
    username: &str,
    key_path: &str,
) -> Result<()> {
    if key_path.trim().is_empty() {
        return Ok(());
    }

    delete_secret(&passphrase_account(host, port, username, key_path))
}
