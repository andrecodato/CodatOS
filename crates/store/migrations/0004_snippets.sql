-- Snippets (RF-CON-023). As variáveis são os `{{nome}}` do próprio comando.
CREATE TABLE snippets (
    id         TEXT PRIMARY KEY,
    tenant_id  TEXT,
    name       TEXT NOT NULL,
    command    TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
