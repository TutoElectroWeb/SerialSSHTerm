# Guide Complet du Packaging (Debian + Windows)

Ce document décrit en détail le processus de création et de distribution des artefacts Debian et Windows pour SerialSSHTerm.

## Structure de packaging

```
SerialSSHTerm/
├── build-deb.sh                    # Script principal de build .deb
├── install-deps.sh                 # Script d'installation des dépendances
├── Cargo.toml                       # Configuration Rust
├── src/                             # Code source
├── assets/
│   └── icon.svg                     # Icône de l'application
└── debian/
    ├── control                      # Métadonnées du paquet
    ├── rules                        # Règles de build
    ├── changelog                    # Historique des versions
    ├── copyright                    # Licence
    ├── compat                       # Version debhelper
    ├── source/format                # Format de source
    └── serial-ssh-term.desktop      # Entrée FDO Desktop
```

## Processus de build

### 1. Installation des dépendances système

Le script `install-deps.sh` automatise cela :

```bash
./install-deps.sh
```

Il détecte la distribution Linux et installe :

- **Build tools** : build-essential, debhelper
- **Rust** : via rustup (si absent)
- **GTK4/Libadwaita** : dev packages
- **SSH** : libssl-dev (requis par `russh`)
- **Outils** : pkg-config

### 2. Compilation

Le script `build-deb.sh` exécute :

```bash
cargo build --release
```

Cela crée un binaire optimisé dans `target/release/serial-ssh-term`.

### 3. Création du paquet avec debuild

Le fichier `debian/rules` définit les étapes :

```makefile
override_dh_auto_build:
    cargo build --release

override_dh_auto_install:
    # Copier le binaire, l'icône, l'entrée .desktop
```

`debuild` (outil Debian) exécute `debian/rules` et crée :

- `serial-ssh-term_1.0.0_amd64.deb` — paquet binaire
- `serial-ssh-term_1.0.0.dsc` — descripteur de source
- `serial-ssh-term_1.0.0.tar.xz` — archive source

### 4. Installation

```bash
sudo dpkg -i serial-ssh-term_1.0.0_amd64.deb
```

## Fichiers de configuration Debian

### debian/control

**Rôle** : Métadonnées du paquet (nom, version, dépendances, description).

Champs clés :

- `Package` : Nom du paquet (serial-ssh-term)
- `Version` : Semver (1.0.0)
- `Architecture` : amd64, i386, arm64, etc.
- `Depends` : Dépendances runtime (libc6, libgtk-4-1, etc.)
- `Maintainer` : Responsable (M@nu)
- `Homepage` : URL du projet
- `Description` : Texte court et long

### debian/rules

**Rôle** : Instructions de build pour debhelper.

Utilise `dh` (debhelper) en mode modernisé (sequencer) +
overrides pour les commandes Rust/cargo.

**Important** : doit être **exécutable** (`chmod +x debian/rules`)

### debian/changelog

**Rôle** : Trace l'historique des versions Debian.

Format stricte :

```
<package> (<version>) <distro>; urgency=<priority>

  * <entry 1>
  * <entry 2>

 -- <author> <date>
```

Généré avec `dch` ou édité manuellement.

### debian/copyright

**Rôle** : Déclaration de licence.

Format DEP-5 (Debian Document Exchange Format).

Déclare :

- Titulaires des droits
- Licence (MIT)
- Fichiers exempts

### debian/compat

**Rôle** : Version minimale de debhelper requise.

Valeur : 13 (compatible Bullseye/Bookworm/Jammy+)

### debian/source/format

**Rôle** : Format du paquet source.

Valeur : `3.0 (native)` pour un projet Debian natif.

### debian/serial-ssh-term.desktop

**Rôle** : Intégration FDO (Freedesktop) — entrée de menu d'applications.

Lance `serial-ssh-term` via :

- Super (Win) → recherche « SerialSSHTerm »
- Affiche l'icône et la description
- Catégories : Utility, System, Network

## Dépendances

### Runtime (debian/control - Depends)

```
libc6           ≥ 2.31      - Bibliothèque standard C
libgtk-4-1      ≥ 4.0       - Framework UI
libadwaita-1    ≥ 0.7       - Design system
libssl3         ≥ 3.x       - TLS/crypto requis pour SSH (via Rustls/OpenSSL stack)
```

### Build (install-deps.sh)

```
rustc, cargo    ≥ 1.70      - Compilateur Rust
libgtk-4-dev                - Headers GTK4
libadwaita-1-dev            - Headers Libadwaita
libssl-dev                  - Headers crypto/TLS
pkg-config                  - Détection de libs
build-essential             - Compilateur C
debhelper, devscripts       - Outils Debian
```

**Note** : Les dépendances Cargo (crates.io) sont vendues dans la build.

## Distribution

### Créer et tester un .deb localement

```bash
./install-deps.sh   # Une fois
./build-deb.sh      # Génère .deb
sudo dpkg -i ../serial-ssh-term_1.0.0*.deb

# Test
serial-ssh-term     # Devrait lancer l'app
```

### Upload vers un PPA (Personal Package Archive)

Si vous gérez un PPA Launchpad :

```bash
# 1. Signer le paquet
debsign <file>.dsc

# 2. Upload
dput ppa:username/ppaname <file>.changes
```

## Troubleshooting

### Erreur : « debuild: command not found »

```bash
sudo apt install devscripts
```

### Erreur : « cargo not found »

```bash
./install-deps.sh   # Installe Rust via rustup
```

### Le paquet crée mais refuse d'installer

Vérifier les dépendances manquantes :

```bash
dpkg -I ../serial-ssh-term_1.0.0*.deb | grep Depends
```

### Linter Lintian a des avertissements

```bash
lintian ../serial-ssh-term_1.0.0*.deb
```

Ignorer les non-critiques (ex. "bad-distribution-in-changes-file" avec Launchpad).

## Formats d'architecture supportés

Le script `build-deb.sh` crée des .deb pour la machine hôte (`uname -m`).

Pour compiler **cross-architecture** (ex. ARM64 sur x86_64) :

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --target aarch64-unknown-linux-gnu --release
```

Puis adapter debian/rules et debian/control.

## Mises à jour futures

Pour publier une nouvelle version (ex. 1.1.0) :

1. Incrémenter la version dans `Cargo.toml`
2. Ajouter une entrée dans `debian/changelog`
3. Relancer `./build-deb.sh`

```bash
dch -i  # Incrémente auto dans debian/changelog
```

## Ressources

- [Debian Packaging Manual](https://www.debian.org/doc/manuals/packaging-tutorial/)
- [debhelper](https://manpages.debian.org/debhelper)
- [Cargo Debian integration](https://rust-lang.org/what/wg-cargo/)
- [FDO Desktop Entry Spec](https://specifications.freedesktop.org/desktop-entry-spec/latest/)

---

## Packaging Windows (.exe)

Le projet fournit un flux équivalent à Debian pour Windows 11.

### Scripts fournis

- `install-deps-windows.ps1` : installe Rust + MSYS2 + toolchain GTK4/libadwaita.
- `build-exe.ps1` : compile l'application et prépare une distribution ZIP.

### Processus recommandé

Dans PowerShell (Windows 11) :

```powershell
powershell -ExecutionPolicy Bypass -File .\install-deps-windows.ps1
powershell -ExecutionPolicy Bypass -File .\build-exe.ps1 -IncludeGtkRuntime
winget install JRSoftware.InnoSetup
powershell -ExecutionPolicy Bypass -File .\build-installer.ps1 -IncludeGtkRuntime
```

### Artefacts générés

- `dist/windows/SerialSSHTerm/serial-ssh-term.exe`
- `dist/windows/serial-ssh-term-win64-release.zip`
- `dist/windows/installer/serial-ssh-term-setup-win64-v<version>.exe`

### Notes importantes

- Le flag `-IncludeGtkRuntime` copie les DLL GTK depuis `C:\msys64\mingw64\bin`.
- Sans ce flag, l'exécutable dépend d'un runtime GTK installé sur la machine cible.
- Pour un packaging MSI/Setup, ce ZIP peut ensuite être encapsulé avec Inno Setup ou WiX.
