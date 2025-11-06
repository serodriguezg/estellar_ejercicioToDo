#!/bin/bash
set -e

echo "=== Instalación automática de Stellar CLI en Ubuntu (Bash) ==="

# 1. Actualizar repositorios e instalar dependencias básicas (sin pedir confirmación)
sudo apt-get update -y
sudo apt-get install -y curl build-essential git

# 2. Instalar Rust mediante rustup en modo desatendido
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# 3. Configurar variable de entorno para que rustup sea reconocible en Bash
source "$HOME/.cargo/env"

# 4. Añadir soporte para el target wasm32v1-none
rustup target add wasm32v1-none

# 5. Descargar e instalar Stellar CLI
wget -q https://github.com/stellar/stellar-cli/releases/download/v23.1.1/stellar-cli-23.1.1-x86_64-unknown-linux-gnu.tar.gz
tar -xvf stellar-cli-23.1.1-x86_64-unknown-linux-gnu.tar.gz
sudo mv stellar /usr/local/bin/
sudo chmod +x /usr/local/bin/stellar
rm stellar-cli-23.1.1-x86_64-unknown-linux-gnu.tar.gz

# 6. Verificar instalaciones
echo "=== Verificando instalación de Rust ==="
rustc --version || { echo "Error: Rust no se instaló correctamente"; exit 1; }
cargo --version || { echo "Error: Cargo no se instaló correctamente"; exit 1; }
rustup --version || { echo "Error: rustup no se instaló correctamente"; exit 1; }

echo "=== Verificando instalación de Stellar CLI ==="
stellar --version || { echo "Error: Stellar CLI no se instaló correctamente"; exit 1; }

echo "=== Verificando instalación de Git ==="
git --version

echo "=== Instalación completada correctamente ==="