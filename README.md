# SerialSSHTerm

Langue : **Français** | [English](README.en.md)

Un terminal professionnel pour connexions série et SSH, écrit en **Rust** avec **GTK4** et **Libadwaita**.

## 🎯 À propos

SerialSSHTerm est une application de terminal unifiée permettant de communiquer avec des appareils via **port série** ou **SSH** dans une interface moderne et épurée. L'application offre :

- 🔌 **Connexion série** configurable (débit, bits de données, parité, arrêt, contrôle de flux)
- 🔐 **Connexion SSH** avec authentification par mot de passe ou clé privée, vérification TOFU des clés hôtes
- 🖥️ **Émulation terminal ANSI** complète (couleurs 256, SGR, séquences d'échappement)
- 📝 **Affichage en temps réel** avec scrollback configurable
- 💾 **Sauvegarde des logs** en fichier texte
- 🎨 **Thèmes** (Clair, Sombre, Hacker)
- ⚙️ **Configuration persistante** en JSON
- 🔐 **Secrets SSH stockés dans le trousseau système** (Secret Service Linux / Credential Manager Windows)
- 🔔 **Notifications toast** Adwaita non-bloquantes
- 🧮 **Outils intégrés** : calculatrice et convertisseur DEC/HEX/BIN

## 🚀 Installation

### Option 1 : Paquet Debian (.deb) — Recommandé

Pour une installation système simple sur Ubuntu/Debian :

```bash
# Installer les dépendances de build automatiquement
./install-deps.sh

# Créer le paquet .deb
./build-deb.sh

# Installer
sudo dpkg -i ../serial-ssh-term_1.0.0*.deb
```

Le paquet inclut l'icône, l'entrée de menu, et la configuration système.

### Option 1 bis : Build Windows (.exe)

Depuis Windows 11 (PowerShell) :

```powershell
# 1) Installer les dépendances (Rust + MSYS2 + GTK4/libadwaita)
powershell -ExecutionPolicy Bypass -File .\install-deps-windows.ps1

# 2) Générer l'exe + archive ZIP distributable
powershell -ExecutionPolicy Bypass -File .\build-exe.ps1 -IncludeGtkRuntime

# 3) Installer Inno Setup (une fois)
winget install JRSoftware.InnoSetup

# 4) Générer un installateur Windows (.exe)
powershell -ExecutionPolicy Bypass -File .\build-installer.ps1 -IncludeGtkRuntime
```

Artefacts générés :

- `dist/windows/SerialSSHTerm/serial-ssh-term.exe`
- `dist/windows/serial-ssh-term-win64-release.zip`
- `dist/windows/installer/serial-ssh-term-setup-win64-v<version>.exe`

### Option 2 : Installation depuis la source

#### Prérequis

- **Rust** (1.75+) — [installer](https://rustup.rs/)
- **GTK 4.14+** et dépendances de développement
- **OpenSSL** dev (requis par `russh`)

#### Installation des dépendances

Automatique :

```bash
./install-deps.sh
```

Ou manuel :

**Ubuntu/Debian** :

```bash
sudo apt update
sudo apt install build-essential libgtk-4-dev libadwaita-1-dev libssl-dev pkg-config cargo
```

**Fedora** :

```bash
sudo dnf install gtk4-devel libadwaita-devel openssl-devel pkg-config cargo
```

**Arch** :

```bash
sudo pacman -Sy gtk4 libadwaita openssl pkg-config rustup
rustup default stable
```

#### Compiler et lancer

```bash
cargo build --release
./target/release/serial-ssh-term
```

Ou directement (mode debug) :

```bash
cargo run
```

## 📖 Utilisation

### Connexion série

1. Sélectionnez l'onglet **🔌 Série**
2. Choisissez le port dans la liste déroulante
3. Configurez les paramètres (vitesse, bits, parité, etc.)
4. Cliquez **Se connecter**
5. Tapez vos commandes et appuyez sur Entrée

### Connexion SSH

1. Sélectionnez l'onglet **🔐 SSH**
2. Entrez l'hôte, le port, l'utilisateur
3. Choisissez l'authentification :
   - **Mot de passe** : saisissez-le directement
   - **Clé privée** : parcourez vers `~/.ssh/id_rsa`
4. Activez/désactivez **Mémoriser secrets** selon votre politique sécurité
5. Cliquez **Se connecter**
6. Si le serveur est inconnu, un dialogue TOFU s'affiche pour confirmer l'empreinte de la clé hôte. En cas de changement de clé détecté, un avertissement MITM est affiché.

### Raccourcis clavier

- **Ctrl+S** : Sauvegarder les logs
- **Ctrl+L** : Effacer le terminal
- **Entrée** (dans le champ) : Envoyer la commande

## ⚙️ Configuration

La configuration est automatiquement sauvegardée dans :

```
~/.config/serial-ssh-term/settings.json
```

Elle inclut :

- Derniers paramètres de connexion (série / SSH)
- Thème actif
- Taille de la fenêtre
- Limite de scrollback
- Fin de ligne (LF / CR / CRLF)

Les secrets (mot de passe SSH, passphrase de clé) ne sont pas écrits dans `settings.json`.
Ils sont enregistrés dans le trousseau système de l'OS.
Le paramètre `remember_secrets` (booléen) pilote cette mémorisation.

## 🛠️ Architecture

L'application suit une architecture en trois couches découplées (**SOLID / DDD**) :

```
┌─────────────────────────────────────────────────┐
│           Application (main.rs)                 │
├─────────────────────────────────────────────────┤
│  UI Layer  (src/ui/)           ← thread GTK     │
│  - window.rs        orchestration + GLib timer  │
│  - terminal_panel.rs  affichage ANSI (vte)      │
│  - connection_panel.rs  configs série / SSH     │
│  - tools_dialog.rs  calculatrice / conv. base   │
│  - input_panel.rs   saisie utilisateur          │
│  - header_bar.rs    menu / statuts              │
│  - theme.rs         thèmes Clair/Sombre/Hacker  │
├─────────────────────────────────────────────────┤
│  Pont async_channel (sans dépendance GTK)       │
│  spawn_connection_actor  →  ConnectionEvent     │
│  GLib::timeout_add_local (20 ms)      ↑         │
├─────────────────────────────────────────────────┤
│  Core Layer  (src/core/)       ← tokio task     │
│  - connection.rs    trait + acteur I/O          │
│  - serial_manager.rs  async tokio-serial        │
│  - ssh_manager.rs   russh async + TOFU          │
│  - settings.rs      persistance JSON            │
│  - logger.rs        fichier de logs             │
└─────────────────────────────────────────────────┘
```

### Modèle acteur

`spawn_connection_actor` exécute toutes les I/O dans une tâche Tokio dédiée et relaie les événements vers GTK via un canal `async_channel` — **aucun blocage du thread UI**.

### Trait `Connection`

Abstraction asynchrone unifiée pour série et SSH :

```rust
#[async_trait]
pub trait Connection: Send {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn send(&mut self, data: &[u8]) -> Result<usize>;
    async fn read(&mut self) -> Result<Vec<u8>>;
    fn state(&self) -> ConnectionState;
    fn connection_type(&self) -> ConnectionType;
    fn description(&self) -> String;
    fn bytes_sent(&self) -> u64;
    fn bytes_received(&self) -> u64;
}
```

### Sécurité SSH — TOFU

La vérification des clés hôtes suit le modèle **TOFU** (_Trust On First Use_) :

- Première connexion → dialogue de confirmation + enregistrement dans `known_hosts`
- Clé changée → avertissement MITM avec bouton destructif rouge
- Timeout 5 min si l'utilisateur ne répond pas

## 📦 Packaging Debian

SerialSSHTerm inclut des scripts pour créer un paquet Debian (.deb) professionnel.

### Fichiers de packaging

```
debian/
├── control           # Métadonnées du paquet
├── changelog         # Historique des versions
├── copyright         # Licence MIT
├── rules             # Instructions de build
├── source/format     # Format Debian 3.0 (native)
└── serial-ssh-term.desktop  # Entrée de menu
assets/
└── icon.svg          # Icône de l'application
build-deb.sh         # Script de création du .deb
build-exe.ps1        # Script de création du .exe (Windows)
build-installer.ps1  # Script de création de l'installateur Windows (.exe)
install-deps.sh      # Script d'installation des dépendances
install-deps-windows.ps1  # Dépendances Windows (Rust + MSYS2 + GTK)
```

### Créer le paquet

```bash
# 1. Installer les dépendances (usage unique)
./install-deps.sh

# 2. Construire le .deb
./build-deb.sh

# 3. Installer le paquet généré
sudo dpkg -i ../serial-ssh-term_1.0.0*.deb
```

Le paquet est créé dans le répertoire parent du projet.

### Contenu du paquet

- **Exécutable** : `/usr/bin/serial-ssh-term`
- **Icône** : `/usr/share/icons/hicolor/scalable/apps/serial-ssh-term.svg`
- **Entrée de menu** : `/usr/share/applications/serial-ssh-term.desktop`

### Déinstallation

```bash
sudo apt remove serial-ssh-term
# ou
sudo dpkg -r serial-ssh-term
```

## 🐛 Dépannage

### Message GTK-CRITICAL : « Unable to connect to the accessibility bus »

Ce message apparaît quand le service **AT-SPI** (Assistive Technology Service Provider Interface) du système n'est pas actif. C'est un service d'accessibilité Linux que GTK4 contacte au démarrage.

**Cause** : Le démon `at-spi-dbus-bus` s'est arrêté (crash, mise à jour système, etc.)

**Solution** :

```bash
systemctl --user restart at-spi-dbus-bus.service
```

Cela relance le service d'accessibilité et supprime le message. L'application fonctionne normalement même sans ce message — il est purement cosmétique et ne reflète aucun dysfonctionnement de SerialSSHTerm.

### L'application ne trouve pas les ports série

- Vérifiez les permissions : `groups | grep dialout`
- Si absent, ajoutez-vous : `sudo usermod -aG dialout $USER`
- Déconnectez-vous et reconnectez-vous

### SSH : « Authentification échouée »

- Vérifiez la clé privée : `ssh-keygen -l -f ~/.ssh/id_rsa`
- Vérifiez l'utilisateur et l'hôte
- Testez manuellement : `ssh user@host`

## 📦 Dépendances

| Crate                  | Rôle                              |
| ---------------------- | --------------------------------- |
| `gtk4`                 | Framework UI GTK4                 |
| `libadwaita`           | Design system GNOME (Adwaita)     |
| `tokio`                | Runtime async                     |
| `tokio-serial`         | I/O série asynchrone              |
| `russh`                | Connexions SSH async + TOFU       |
| `async-channel`        | Canal UI ↔ acteur I/O             |
| `async-trait`          | Trait asynchrone                  |
| `vte`                  | Parseur ANSI / séquences terminal |
| `serialport`           | Énumération des ports série       |
| `serde` / `serde_json` | Sérialisation config              |
| `chrono`               | Timestamps                        |
| `log` / `env_logger`   | Logging                           |
| `anyhow`               | Gestion d'erreurs                 |
| `keyring`              | Trousseau système (secrets SSH)   |
| `dirs`                 | Répertoires XDG                   |
| `meval`                | Évaluation d'expressions (outils) |

## 📄 Licence

[MIT](LICENSE)

## 👤 Auteur

Créé par **M@nu** — [GitHub](https://github.com/weedmanu/SerialSSHTerm)

---

**Note** : Ce projet est produit avec soin — architecture acteur async, 0 warning clippy strict, émulation ANSI, SSH TOFU, 0 code mort. Prêt pour l'utilisation quotidienne.
