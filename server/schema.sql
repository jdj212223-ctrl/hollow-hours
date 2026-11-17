CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  email TEXT UNIQUE NOT NULL,          -- lower-cased
  name TEXT UNIQUE NOT NULL,           -- display name shown in game
  salt TEXT NOT NULL,                  -- hex
  hash TEXT NOT NULL,                  -- PBKDF2-SHA256, hex
  created INTEGER NOT NULL,
  failed INTEGER NOT NULL DEFAULT 0,   -- wrong passwords in a row
  locked_until INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS sessions (
  token_hash TEXT PRIMARY KEY,         -- SHA-256 of the token; the token itself is never stored
  user_id INTEGER NOT NULL,
  expires INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS servers (
  code TEXT PRIMARY KEY,               -- room code
  name TEXT NOT NULL,
  owner INTEGER NOT NULL,
  public INTEGER NOT NULL DEFAULT 1,
  max INTEGER NOT NULL DEFAULT 8,
  mods TEXT NOT NULL DEFAULT '[]',     -- JSON array of mod ids
  players INTEGER NOT NULL DEFAULT 0,
  created INTEGER NOT NULL,
  seen INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS mods (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  version TEXT NOT NULL,
  author INTEGER NOT NULL,
  description TEXT,
  content TEXT NOT NULL,               -- validated content.json
  created INTEGER NOT NULL
);
-- v3: outside servers (hosted somewhere else). address = wss://..., secret_hash = SHA-256 of the server's heartbeat secret.
-- ALTER TABLE servers ADD COLUMN address TEXT;
-- ALTER TABLE servers ADD COLUMN secret_hash TEXT;

-- v4: logging the game in through the browser (one-time codes with PKCE)
CREATE TABLE IF NOT EXISTS link_codes (code_hash TEXT PRIMARY KEY, user_id INTEGER NOT NULL, challenge TEXT NOT NULL, expires INTEGER NOT NULL, used INTEGER NOT NULL DEFAULT 0);

-- v5: privacy
-- ALTER TABLE users ADD COLUMN hide_host INTEGER NOT NULL DEFAULT 0; ALTER TABLE users ADD COLUMN hide_author INTEGER NOT NULL DEFAULT 0;
