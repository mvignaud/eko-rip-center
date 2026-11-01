import * as Crypto from 'expo-crypto';
import * as Device from 'expo-device';
import * as WebBrowser from 'expo-web-browser';
import { Auth, ErreurApi, MESSAGE_RESEAU, stockage } from './api';

// Cloud E-KO RIP : connexion par OAuth 2 (code d'autorisation + PKCE) sur la plate-forme,
// puis accès aux serveurs RIP du compte à travers le relais, en `Authorization: Bearer`.
// Protocole : docs/fr/cloud.md, « Se connecter au cloud depuis les apps ».

export const PLATEFORME = 'https://platform.e-ko.rip';
const CLIENT_ID = 'eko-rip-center';
const RETOUR = 'ekorip://oauth';
const DELAI_MS = 15000;

export interface SiteCloud {
  id: string;
  nom: string;
  /** Adresse du serveur RIP à travers le relais (https://<site>.platform.e-ko.rip). */
  adresse: string;
  en_ligne: boolean;
  role: string;
}

export interface MoiCloud {
  compte: { email: string; nom: string };
  sites: SiteCloud[];
}

function base64url(b64: string): string {
  return b64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function hex(octets: Uint8Array): string {
  return Array.from(octets, (o) => o.toString(16).padStart(2, '0')).join('');
}

function parametres(url: string): Record<string, string> {
  const q = url.split('?')[1]?.split('#')[0] ?? '';
  const r: Record<string, string> = {};
  for (const p of q.split('&')) {
    if (!p) continue;
    const [k, v = ''] = p.split('=');
    r[decodeURIComponent(k)] = decodeURIComponent(v.replace(/\+/g, ' '));
  }
  return r;
}

function formulaire(champs: Record<string, string>): string {
  return Object.entries(champs).map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`).join('&');
}

async function appel(methode: string, chemin: string, jeton: string | null, corps?: string, type?: string): Promise<Response> {
  const ctrl = new AbortController();
  const minuteur = setTimeout(() => ctrl.abort(), DELAI_MS);
  try {
    return await fetch(`${PLATEFORME}${chemin}`, {
      method: methode,
      credentials: 'omit',
      signal: ctrl.signal,
      headers: {
        Accept: 'application/json',
        ...(jeton ? { Authorization: `Bearer ${jeton}` } : {}),
        ...(type ? { 'Content-Type': type } : {}),
      },
      body: corps,
    });
  } catch {
    throw new ErreurApi(0, 'reseau', MESSAGE_RESEAU);
  } finally {
    clearTimeout(minuteur);
  }
}

async function erreur(r: Response): Promise<ErreurApi> {
  let code = `http_${r.status}`;
  try {
    const j = await r.json();
    code = j?.code ?? j?.error ?? code;
  } catch { /* corps non JSON */ }
  return new ErreurApi(r.status, code, '');
}

/**
 * Ouvre la plate-forme dans le navigateur (mot de passe, Google ou Facebook), puis échange le
 * code contre un jeton d'app (180 jours), gardé dans le SecureStore. `null` si annulé.
 */
export async function connecterCloud(): Promise<string | null> {
  const verificateur = hex(Crypto.getRandomBytes(32)); // 64 caractères [0-9a-f], admis par RFC 7636
  const defi = base64url(await Crypto.digestStringAsync(
    Crypto.CryptoDigestAlgorithm.SHA256, verificateur, { encoding: Crypto.CryptoEncoding.BASE64 }));
  const etat = Crypto.randomUUID();
  const appareil = Device.deviceName ?? Device.modelName ?? 'Téléphone';
  const url = `${PLATEFORME}/oauth/autoriser?${formulaire({
    client_id: CLIENT_ID, response_type: 'code', redirect_uri: RETOUR,
    code_challenge: defi, code_challenge_method: 'S256', state: etat, appareil: `App mobile (${appareil})`,
  })}`;

  const res = await WebBrowser.openAuthSessionAsync(url, RETOUR);
  if (res.type !== 'success') return null;
  const p = parametres(res.url);
  if (p.state !== etat) throw new ErreurApi(400, 'cloud_etat', '');
  if (p.error === 'access_denied') return null;
  if (!p.code) throw new ErreurApi(400, p.error || 'cloud_code', '');

  const r = await appel('POST', '/oauth/jeton', null, formulaire({
    grant_type: 'authorization_code', client_id: CLIENT_ID, code: p.code, redirect_uri: RETOUR, code_verifier: verificateur,
  }), 'application/x-www-form-urlencoded');
  if (!r.ok) throw await erreur(r);
  const { access_token: jeton } = (await r.json()) as { access_token: string };
  await stockage.ecrire('cloud_jeton', jeton);
  return jeton;
}

/** Compte et sites du jeton. */
export async function moi(jeton: string): Promise<MoiCloud> {
  const r = await appel('GET', '/api/moi', jeton);
  if (!r.ok) throw await erreur(r);
  return (await r.json()) as MoiCloud;
}

/**
 * Inscrit le téléphone aux notifications de la plate-forme (site hors ligne, de retour,
 * nouvelle app connectée). Sans effet si la plate-forme ne connaît pas encore la route.
 */
export async function inscrirePushCloud(jeton: string, jetonExpo: string, plateforme: 'ios' | 'android'): Promise<void> {
  try {
    await appel('POST', '/api/moi/push', jeton, JSON.stringify({ jeton_expo: jetonExpo, plateforme }), 'application/json');
  } catch { /* plate-forme injoignable : réessayé à la prochaine connexion */ }
}

/** Révoque le jeton d'app sur la plate-forme et l'oublie. */
export async function deconnecterCloud(): Promise<void> {
  const jeton = await stockage.lire('cloud_jeton');
  await stockage.ecrire('cloud_jeton', null);
  if (jeton) appel('POST', '/api/moi/deconnexion', jeton).catch(() => {});
}

/** Requêtes vers un serveur RIP à travers le relais : Bearer, pas de cookie, pas de reconnexion. */
export class AuthCloud implements Auth {
  constructor(private jeton: string) {}
  async entetes() { return { Authorization: `Bearer ${this.jeton}` }; }
  async renouveler() { return false; }
}
