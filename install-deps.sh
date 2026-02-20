#!/bin/bash
#
# Script d'installation des dépendances pour SerialSSHTerm
#
# Ce script installe les dépendances de build et runtime nécessaires
# pour compiler et utiliser SerialSSHTerm sur Ubuntu/Debian.
#

set -e

echo "═══════════════════════════════════════════════════════════"
echo "  SerialSSHTerm - Installation des dépendances"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Détecter la distribution
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO=$ID
else
    echo "✗ Impossible de détecter la distribution"
    exit 1
fi

echo "📦 Distribution détectée : $DISTRO"
echo ""

# Installer les dépendances selon la distribution
case "$DISTRO" in
    ubuntu|debian)
        echo "📥 Installation des dépendances pour Debian/Ubuntu..."
        echo ""
        
        # Mettre à jour les listes
        echo "↻ Mise à jour des listes de paquets..."
        sudo apt update
        
        # Build essentials + outils Debian packaging
        echo ""
        echo "📦 Build essentials & packaging..."
        sudo apt install -y build-essential debhelper devscripts lintian
        
        # Rust (si pas présent)
        echo ""
        echo "📦 Vérification de Rust..."
        if ! command -v cargo &> /dev/null; then
            echo "   Rust n'est pas installé. Installation de rustup..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        else
            echo "   ✓ Rust est déjà installé"
        fi
        
        # Dépendances GTK4
        echo ""
        echo "📦 Dépendances GTK4..."
        sudo apt install -y libgtk-4-dev libadwaita-1-dev
        
        # SSH
        echo ""
        echo "📦 Dépendances SSH (OpenSSL)..."
        sudo apt install -y libssl-dev
        
        # Outils de build
        echo ""
        echo "📦 Outils..."
        sudo apt install -y pkg-config libudev-dev
        
        ;;
    fedora)
        echo "📥 Installation des dépendances pour Fedora..."
        echo ""
        
        echo "↻ Mise à jour..."
        sudo dnf upgrade -y
        
        echo "📦 Build essentials..."
        sudo dnf groupinstall -y "Development Tools"
        sudo dnf install -y rpm-build
        
        if ! command -v cargo &> /dev/null; then
            echo "   Installation de rustup..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        fi
        
        echo "📦 Dépendances GTK4..."
        sudo dnf install -y gtk4-devel libadwaita-devel
        
        echo "📦 Dépendances SSH (OpenSSL)..."
        sudo dnf install -y openssl-devel
        
        echo "📦 Outils..."
        sudo dnf install -y pkg-config
        
        ;;
    arch)
        echo "📥 Installation des dépendances pour Arch..."
        echo ""
        
        echo "📦 Build essentials..."
        sudo pacman -Sy --noconfirm base-devel
        
        if ! command -v cargo &> /dev/null; then
            echo "Installation de rustup..."
            sudo pacman -Sy --noconfirm rustup
            rustup default stable
        fi
        
        echo "📦 Dépendances GTK4..."
        sudo pacman -Sy --noconfirm gtk4 libadwaita
        
        echo "📦 Dépendances SSH (OpenSSL)..."
        sudo pacman -Sy --noconfirm openssl
        
        echo "📦 Outils..."
        sudo pacman -Sy --noconfirm pkg-config
        
        ;;
    *)
        echo "⚠ Distribution non reconnue : $DISTRO"
        echo ""
        echo "Installation manuelle requise. Installez :"
        echo "  - build-essential ou equivalent"
        echo "  - Rust (via rustup)"
        echo "  - libgtk-4-dev libadwaita-1-dev (ou équivalent)"
        echo "  - libssl-dev (ou équivalent OpenSSL)"
        echo "  - pkg-config"
        exit 1
        ;;
esac

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "✓ Toutes les dépendances sont installées !"
echo ""
echo "Étapes suivantes :"
echo "  1. cd $(dirname "$0")"
echo "  2. cargo build --release"
echo "  3. ./target/release/serial-ssh-term"
echo ""
echo "Pour créer un paquet .deb :"
echo "  ./build-deb.sh"
echo "═══════════════════════════════════════════════════════════"
