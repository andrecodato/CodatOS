#!/usr/bin/env bash
# Prepara o Ubuntu (WSL) para desenvolver o CodatOS. Executar como root.
# Uso: setup-root.sh <usuario-de-desenvolvimento>
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

DEV_USER="${1:?informe o usuario de desenvolvimento}"
NODE_MAJOR=24
. /etc/os-release
CODENAME="${VERSION_CODENAME:-${UBUNTU_CODENAME:-}}"
ARCH="$(dpkg --print-architecture)"
echo "==> Ubuntu ${VERSION_ID} (${CODENAME}, ${ARCH}) — usuario ${DEV_USER}"

echo "==> apt update"
apt-get update -q

echo "==> pacotes base e git"
apt-get install -y -q build-essential pkg-config libssl-dev curl wget file ca-certificates gnupg unzip git

echo "==> dependencias do Tauri (Linux) e toolchain para cross-compilar o app Windows"
apt-get install -y -q libwebkit2gtk-4.1-dev libxdo-dev libayatana-appindicator3-dev librsvg2-dev clang lld llvm nsis

install -m 0755 -d /etc/apt/keyrings

echo "==> repositorio NodeSource (Node.js ${NODE_MAJOR}.x)"
curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key \
  | gpg --dearmor --yes -o /etc/apt/keyrings/nodesource.gpg
echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_${NODE_MAJOR}.x nodistro main" \
  > /etc/apt/sources.list.d/nodesource.list

echo "==> repositorio oficial da Docker"
if curl -fsI "https://download.docker.com/linux/ubuntu/dists/${CODENAME}/Release" >/dev/null 2>&1; then
  curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
  chmod a+r /etc/apt/keyrings/docker.asc
  echo "deb [arch=${ARCH} signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu ${CODENAME} stable" \
    > /etc/apt/sources.list.d/docker.list
  DOCKER_PKGS="docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin"
else
  echo "    o repositorio da Docker ainda nao publica '${CODENAME}'; usando os pacotes do Ubuntu"
  DOCKER_PKGS="docker.io docker-compose-v2 docker-buildx"
fi

echo "==> apt update (novos repositorios) e instalacao de Node.js + Docker"
apt-get update -q
# shellcheck disable=SC2086
apt-get install -y -q nodejs $DOCKER_PKGS

echo "==> pnpm (global via npm)"
npm install -g pnpm@latest --no-fund --no-audit

echo "==> servico do Docker e grupo 'docker' para ${DEV_USER}"
systemctl enable --now docker
usermod -aG docker "$DEV_USER"

echo "==> concluido"
