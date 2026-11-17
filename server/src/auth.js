// Accounts: email + password + display name.
//  - Passwords: PBKDF2-SHA256 (100,000 rounds) with a random salt per user. The password itself is never stored.
//  - Sessions: a random 32-byte token, given to the browser (HttpOnly cookie) or the game (Bearer). Only its SHA-256
//    is stored, so a database leak doesn't give working logins. 30 days.
//  - 10 wrong passwords in a row lock the account for 15 minutes.

const hex = (b) => [...new Uint8Array(b)].map((x) => x.toString(16).padStart(2, "0")).join("");
const enc = new TextEncoder();
const now = () => Math.floor(Date.now() / 1000);

export const validEmail = (e) => typeof e === "string" && e.length <= 200 && /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(e);
export const validName = (n) => typeof n === "string" && /^[A-Za-z0-9_-]{3,16}$/.test(n);
export const validPassword = (p) => typeof p === "string" && p.length >= 8 && p.length <= 128;

async function pbkdf2(password, saltHex) {
  const salt = Uint8Array.from(saltHex.match(/../g).map((h) => parseInt(h, 16)));
  const key = await crypto.subtle.importKey("raw", enc.encode(password), "PBKDF2", false, ["deriveBits"]);
  return hex(await crypto.subtle.deriveBits({ name: "PBKDF2", hash: "SHA-256", salt, iterations: 100000 }, key, 256));
}

const sha256 = async (s) => hex(await crypto.subtle.digest("SHA-256", enc.encode(s)));

/** Compares two hex strings in constant time. */
function same(a, b) {
  if (a.length !== b.length) return false;
  let d = 0;
  for (let i = 0; i < a.length; i++) d |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return d === 0;
}

async function newSession(env, userId) {
  const token = hex(crypto.getRandomValues(new Uint8Array(32)));
  await env.DB.prepare("INSERT INTO sessions (token_hash, user_id, expires) VALUES (?, ?, ?)").bind(await sha256(token), userId, now() + 30 * 86400).run();
  return token;
}

export async function register(env, email, password, name) {
  email = String(email || "").trim().toLowerCase();
  name = String(name || "").trim();
  if (!validEmail(email)) return { error: "That doesn't look like an email address." };
  if (!validName(name)) return { error: "Display name: 3-16 letters, numbers, _ or -." };
  if (!validPassword(password)) return { error: "Password: at least 8 characters." };
  const taken = await env.DB.prepare("SELECT email, name FROM users WHERE email = ? OR lower(name) = lower(?)").bind(email, name).first();
  if (taken) return { error: taken.email === email ? "There's already an account with that email." : "That display name is taken." };
  const salt = hex(crypto.getRandomValues(new Uint8Array(16)));
  const r = await env.DB.prepare("INSERT INTO users (email, name, salt, hash, created) VALUES (?, ?, ?, ?, ?)")
    .bind(email, name, salt, await pbkdf2(password, salt), now()).run();
  return { token: await newSession(env, r.meta.last_row_id), name };
}

export async function login(env, email, password) {
  email = String(email || "").trim().toLowerCase();
  const u = await env.DB.prepare("SELECT * FROM users WHERE email = ?").bind(email).first();
  // Same answer whether the email exists or not.
  const nope = { error: "Wrong email or password." };
  if (!u) { await pbkdf2(String(password || "x"), "00112233445566778899aabbccddeeff"); return nope; }
  if (u.locked_until > now()) return { error: "Too many wrong passwords. Try again in 15 minutes." };
  if (!same(await pbkdf2(String(password || ""), u.salt), u.hash)) {
    const failed = u.failed + 1;
    await env.DB.prepare("UPDATE users SET failed = ?, locked_until = ? WHERE id = ?").bind(failed >= 10 ? 0 : failed, failed >= 10 ? now() + 900 : 0, u.id).run();
    return nope;
  }
  await env.DB.prepare("UPDATE users SET failed = 0 WHERE id = ?").bind(u.id).run();
  return { token: await newSession(env, u.id), name: u.name };
}

/** The logged-in user from the cookie (website) or `Authorization: Bearer` (game), or null. */
export async function currentUser(env, req) {
  const auth = req.headers.get("Authorization") || "";
  let token = auth.startsWith("Bearer ") ? auth.slice(7).trim() : null;
  if (!token) token = (req.headers.get("Cookie") || "").match(/(?:^|;\s*)hh_session=([0-9a-f]{64})/)?.[1] ?? null;
  if (!token || !/^[0-9a-f]{64}$/.test(token)) return null;
  return env.DB.prepare("SELECT u.id, u.name, u.email FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = ? AND s.expires > ?")
    .bind(await sha256(token), now()).first();
}

export async function logout(env, req) {
  const token = (req.headers.get("Cookie") || "").match(/(?:^|;\s*)hh_session=([0-9a-f]{64})/)?.[1];
  if (token) await env.DB.prepare("DELETE FROM sessions WHERE token_hash = ?").bind(await sha256(token)).run();
}

export const sessionCookie = (token) => `hh_session=${token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=${30 * 86400}`;
export const clearCookie = "hh_session=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0";
