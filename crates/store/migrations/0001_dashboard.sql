-- Preferências, dashboards e widgets (docs/05-modelo-de-dados.md, "Núcleo e preferências").
-- tenant_id fica nulo até os tenants chegarem (M3).

CREATE TABLE settings (
    key        TEXT NOT NULL,
    scope      TEXT NOT NULL CHECK (scope IN ('global', 'device')),
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (key, scope)
) STRICT;

CREATE TABLE dashboards (
    id         TEXT PRIMARY KEY,
    tenant_id  TEXT,
    name       TEXT NOT NULL,
    position   INTEGER NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE TABLE widgets (
    id           TEXT PRIMARY KEY,
    dashboard_id TEXT NOT NULL REFERENCES dashboards (id) ON DELETE CASCADE,
    kind         TEXT NOT NULL,
    layout       TEXT NOT NULL,
    config       TEXT NOT NULL,
    position     INTEGER NOT NULL
) STRICT;

CREATE INDEX widgets_dashboard ON widgets (dashboard_id, position);
