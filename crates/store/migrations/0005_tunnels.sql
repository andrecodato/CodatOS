-- Túneis (RF-CON-040). Somem junto com o host.
CREATE TABLE tunnels (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT,
    host_id     TEXT NOT NULL REFERENCES hosts (id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('local', 'remote', 'dynamic')),
    bind_host   TEXT NOT NULL,
    bind_port   INTEGER NOT NULL CHECK (bind_port BETWEEN 0 AND 65535),
    target_host TEXT NOT NULL DEFAULT '',
    target_port INTEGER NOT NULL DEFAULT 0 CHECK (target_port BETWEEN 0 AND 65535),
    auto_start  INTEGER NOT NULL DEFAULT 0,
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE INDEX tunnels_host ON tunnels (host_id);
