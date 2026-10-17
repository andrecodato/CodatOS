-- Conexões (docs/05-modelo-de-dados.md, "Conexões"). tenant_id fica nulo até o M3.
-- Segredos ficam em colunas *_sealed: envelope JSON do cofre (crate codatos-vault), com o
-- id da linha na AAD, para um envelope não servir em outra linha.

CREATE TABLE host_groups (
    id         TEXT PRIMARY KEY,
    tenant_id  TEXT,
    parent_id  TEXT REFERENCES host_groups (id) ON DELETE SET NULL,
    name       TEXT NOT NULL,
    defaults   TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

-- Chaves SSH (RF-CON-011); geração e importação vêm na PR seguinte.
CREATE TABLE ssh_keys (
    id                 TEXT PRIMARY KEY,
    tenant_id          TEXT,
    name               TEXT NOT NULL,
    algorithm          TEXT NOT NULL,
    public_key         TEXT NOT NULL,
    fingerprint        TEXT NOT NULL,
    private_key_sealed TEXT NOT NULL,
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE TABLE identities (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT,
    name            TEXT NOT NULL,
    username        TEXT NOT NULL,
    password_sealed TEXT,
    key_id          TEXT REFERENCES ssh_keys (id) ON DELETE SET NULL,
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE TABLE hosts (
    id           TEXT PRIMARY KEY,
    tenant_id    TEXT,
    group_id     TEXT REFERENCES host_groups (id) ON DELETE SET NULL,
    name         TEXT NOT NULL,
    address      TEXT NOT NULL,
    port         INTEGER NOT NULL CHECK (port BETWEEN 1 AND 65535),
    protocol     TEXT NOT NULL,
    identity_id  TEXT REFERENCES identities (id) ON DELETE SET NULL,
    jump_host_id TEXT REFERENCES hosts (id) ON DELETE SET NULL,
    tags         TEXT NOT NULL DEFAULT '[]',
    favorite     INTEGER NOT NULL DEFAULT 0,
    color        TEXT,
    notes        TEXT NOT NULL DEFAULT '',
    advanced     TEXT NOT NULL DEFAULT '{}',
    last_used_at TEXT,
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE INDEX hosts_group ON hosts (group_id);

-- Fingerprints aceitas (RF-CON-013): `host:porta` e o tipo da chave do servidor.
CREATE TABLE known_hosts (
    host_pattern  TEXT NOT NULL,
    key_type      TEXT NOT NULL,
    fingerprint   TEXT NOT NULL,
    first_seen_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (host_pattern, key_type)
) STRICT;
