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
