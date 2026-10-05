// Cybermanju Drive — Supabase Auth bridge (static/offline builds)
//
// Why this exists: PKCE OAuth from a pure static page is impossible against
// the providers directly (GitHub demands `client_secret` + serves no CORS;
// Google's token endpoint is equally browser-hostile). Supabase Auth acts
// as the broker: OUR static page talks only to Supabase (CORS-open), and
// Supabase's server does the secret-holding exchange with the provider.
//
// Flow (all standard supabase-js v2 PKCE):
//  1. `startSupabaseOAuth()` — `signInWithOAuth({ skipBrowserRedirect: true })`
//     returns the Supabase authorize URL (PKCE verifier stashed in
//     localStorage by the client). We open it in a popup.
//  2. User approves at the provider → Supabase → redirectTo (our page,
//     `?oauth=popup`) lands *inside the popup*.
//  3. The popup boots this same app; `finishSupabaseReturn()` (App.vue boot)
//     lets the client auto-exchange `?code=` (`detectSessionInUrl`), reads
//     `session.provider_token`, stashes it, and closes the popup.
//  4. The opener polls `supabaseSession()` until `provider_token` appears,
//     saves it into the provider config, and probes — CONNECTED.
//
// The provider token is a real provider credential (GitHub honors it as a
// token; Drive accepts the Bearer). Requested scopes are the minimum the
// sync backends need. Token expiry: GitHub OAuth tokens don't expire by
// default; Google access tokens last ~1h (no browser-safe refresh — the
// panel asks for a reconnect when Google probes fail).

import type { Session, SupabaseClient } from '@supabase/supabase-js'

const URL_KEY = 'cybermanju.supabaseUrl'
const KEY_KEY = 'cybermanju.supabaseKey'
const PENDING_CFG_KEY = 'cybermanju.oauthConfigId'
const TOKEN_STASH_KEY = 'cybermanju.providerToken'

export type OAuthBackend = 'github' | 'google' | 'gitlab'

export interface ProviderTokenStash {
  backend: OAuthBackend
  providerToken: string
  providerRefreshToken?: string | null
  at: number
}

let client: SupabaseClient | null = null
let clientKey = ''

function readLS(key: string): string {
  try {
    return localStorage.getItem(key) || ''
  } catch {
    return ''
  }
}

function writeLS(key: string, value: string) {
  try {
    if (value) localStorage.setItem(key, value)
    else localStorage.removeItem(key)
  } catch {
    // Private mode — session-only.
  }
}

export function getSupabaseConfig(): { url: string; key: string } {
  return { url: readLS(URL_KEY).replace(/\/+$/, ''), key: readLS(KEY_KEY) }
}

export function supabaseConfigured(): boolean {
  const { url, key } = getSupabaseConfig()
  return url.startsWith('http') && key.length > 0
}

export function setSupabaseConfig(url: string, key: string) {
  writeLS(URL_KEY, url.trim().replace(/\/+$/, ''))
  writeLS(KEY_KEY, key.trim())
  client = null
  clientKey = ''
}

export function clearSupabaseConfig() {
  writeLS(URL_KEY, '')
  writeLS(KEY_KEY, '')
  client = null
  clientKey = ''
}

export async function getSupabaseClient(): Promise<SupabaseClient | null> {
  const { url, key } = getSupabaseConfig()
  if (!url || !key) return null
  const cacheKey = `${url}|${key.slice(0, 8)}`
  if (!client || clientKey !== cacheKey) {
    const { createClient } = await import('@supabase/supabase-js')
    client = createClient(url, key, {
      auth: { flowType: 'pkce', detectSessionInUrl: true },
    })
    clientKey = cacheKey
  }
  return client
}

/** Frontend sync backend → Supabase provider slug. */
export function supabaseProviderFor(backendType: string): OAuthBackend | null {
  switch (backendType) {
    case 'github':
      return 'github'
    case 'googleDrive':
    case 'googlePhotos':
      return 'google'
    case 'gitlab':
      return 'gitlab'
    default:
      return null
  }
}

/** Minimum scopes the sync backends need from each provider. */
export function supabaseScopesFor(backendType: string): string {
  switch (backendType) {
    case 'github':
      return 'repo read:user user:email'
    case 'googleDrive':
      return 'openid email profile https://www.googleapis.com/auth/drive.file'
    case 'googlePhotos':
      return 'openid email profile https://www.googleapis.com/auth/photoslibrary.appendonly'
    case 'gitlab':
      return 'api'
    default:
      return ''
  }
}

export function setPendingOAuthConfig(configId: string | null) {
  writeLS(PENDING_CFG_KEY, configId ?? '')
}

export function getPendingOAuthConfig(): string {
  return readLS(PENDING_CFG_KEY)
}

export function stashProviderToken(stash: ProviderTokenStash) {
  writeLS(TOKEN_STASH_KEY, JSON.stringify(stash))
}

export function takeProviderTokenStash(): ProviderTokenStash | null {
  let raw = ''
  try {
    raw = localStorage.getItem(TOKEN_STASH_KEY) || ''
    localStorage.removeItem(TOKEN_STASH_KEY)
  } catch {
    return null
  }
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as ProviderTokenStash
    if (!parsed?.providerToken) return null
    // Stale (older than 10 min) — the code lives 5 min; be generous.
    if (Date.now() - (parsed.at || 0) > 10 * 60 * 1000) return null
    return parsed
  } catch {
    return null
  }
}

/**
 * Begin PKCE OAuth without leaving the app: returns the Supabase authorize
 * URL for us to open in a popup. Throws when Supabase is not configured.
 */
export async function startSupabaseOAuth(backendType: string): Promise<{ url: string }> {
  const provider = supabaseProviderFor(backendType)
  if (!provider) throw new Error(`unsupported: no Supabase OAuth for '${backendType}'`)
  const sb = await getSupabaseClient()
  if (!sb) throw new Error('Supabase is not configured — set URL + key in Settings first')
  const redirectTo = `${window.location.origin}${window.location.pathname}?oauth=popup`
  const { data, error } = await sb.auth.signInWithOAuth({
    provider,
    options: {
      redirectTo,
      scopes: supabaseScopesFor(backendType) || undefined,
      skipBrowserRedirect: true,
    },
  })
  if (error || !data?.url) throw new Error(error?.message || 'Supabase did not return an authorize URL')
  return { url: data.url }
}

export async function supabaseSession(): Promise<Session | null> {
  const sb = await getSupabaseClient()
  if (!sb) return null
  const { data, error } = await sb.auth.getSession()
  if (error) return null
  return data.session
}

export async function supabaseSignOut(): Promise<void> {
  const sb = await getSupabaseClient()
  if (!sb) return
  await sb.auth.signOut()
  client = null
}

/**
 * Called once at app boot. If this page load IS an OAuth return (`?code=`,
 * Supabase configured), the client auto-exchanges it (`detectSessionInUrl`)
 * — here we just read the session, stash the provider token, clean the URL,
 * and close popup returns. Returns true when it handled a return.
 */
export async function finishSupabaseReturn(): Promise<boolean> {
  let params: URLSearchParams
  try {
    params = new URLSearchParams(window.location.search)
  } catch {
    return false
  }
  if (!params.get('code')) return false
  if (!supabaseConfigured()) {
    // A code we can never exchange — strip it so it doesn't linger.
    params.delete('code')
    window.history.replaceState(null, '', `${window.location.pathname}${params.toString() ? `?${params}` : ''}`)
    return true
  }
  try {
    // `detectSessionInUrl` auto-exchanges `?code=` during client creation,
    // but the exchange races this read — one retry before giving up.
    let session = await supabaseSession()
    if (!session?.provider_token) {
      await new Promise((r) => setTimeout(r, 1200))
      session = await supabaseSession()
    }
    const token = session?.provider_token
    if (token) {
      const provider = providerFromSession(session) ?? 'github'
      stashProviderToken({
        backend: provider,
        providerToken: token,
        providerRefreshToken: session?.provider_refresh_token ?? null,
        at: Date.now(),
      })
    }
  } catch {
    // Exchange failed (expired code, verifier mismatch) — still clean up.
  }
  params.delete('code')
  const clean = `${window.location.pathname}${params.toString() ? `?${params}` : ''}${window.location.hash}`
  window.history.replaceState(null, '', clean)
  try {
    if (window.opener && !window.opener.closed) window.close()
  } catch {
    // Cross-origin opener — the main window polls the session anyway.
  }
  return true
}

function providerFromSession(session: Session | null): OAuthBackend | null {
  const provider = (session?.user?.app_metadata as Record<string, unknown> | undefined)?.provider
  if (provider === 'github' || provider === 'google' || provider === 'gitlab') return provider
  return null
}
