CREATE TABLE IF NOT EXISTS exchanges (
  id TEXT PRIMARY KEY,
  request TEXT NOT NULL,
  reply TEXT,
  receipt TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS exchanges_pending ON exchanges(created_at) WHERE reply IS NULL;
