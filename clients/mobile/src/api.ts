import Constants from 'expo-constants';
import * as SecureStore from 'expo-secure-store';
import { reinitialiserDemo, reponseDemo } from './demo';

// ---------- Types ----------

export type Operation = 'impression' | 'decoupe';
export type EtatSortie =
  | 'en_attente' | 'suspendue' | 'en_cours' | 'terminee' | 'echec' | 'annulee' | 'imbriquee';

export interface Sortie {
  id: string;
  machine: string;
  operation: Operation;
  travail: string;
  nom: string;
  page: number;
  etat: EtatSortie;
  code_erreur: string | null;
  erreur: string | null;
  cree_le: number;
  debut: number | null;
  fin: number | null;
  archivee: boolean;
  duree_min: number | null;
  raison_attente: string | null;
  planche_nom: string | null;
  fiche: Record<string, unknown>;
}

export type ActionSortie = 'reprendre' | 'suspendre' | 'annuler' | 'refaire' | 'en_tete';

export interface Encre { encre: string; pourcent: number }

export interface Machine {
  id: string;
  nom: string;
  modele: string;
  operations: Operation[];
  snmp: boolean;
  joignable: boolean | null;
  releve_le: number | null;
  encres: Encre[];
  longueur_restante_m: number | null;
  alertes: string[];
  en_cours: { sortie: string; nom: string; page: number; debut: number | null } | null;
  en_attente: number;
  suspendues: number;
}

export interface EtatMobile {
  version: string;
  dernier_evenement: number;
  machines: Machine[];
}

export type GenreEvenement =
  | 'alerte_machine' | 'encre_basse' | 'encre_vide' | 'machine_injoignable'
  | 'machine_revenue' | 'sortie_echec' | 'sortie_terminee' | 'rendu_echec';

export interface Evenement {
  id: number;
  le: number;
  genre: GenreEvenement;
  gravite: 'erreur' | 'alerte' | 'info';
  machine: string | null;
  titre: string;
  texte: string;
  sortie: string | null;
  travail: string | null;
}

export interface InfoSession {
  acces_libre: boolean;
  utilisateur: string | null;
  role: 'admin' | 'operateur' | null;
}

// ---------- Erreurs ----------

export const MESSAGE_RESEAU =
  "Serveur injoignable — êtes-vous sur le réseau de l'atelier (ou le VPN) ?";

export class ErreurApi extends Error {
  constructor(public statut: number, public code: string, public detail: string) {
    super(detail || code);
  }
  get reseau() { return this.statut === 0; }
}

/** Codes d'erreur du serveur les plus courants dans l'app. */
const MESSAGES: Record<string, string> = {
  action_impossible: "Action impossible dans l'état actuel de la sortie.",
  identifiants_invalides: 'Nom ou mot de passe incorrect.',
  trop_d_echecs: 'Trop de tentatives : réessayez dans 5 minutes.',
  connexion_requise: 'Session expirée : reconnectez-vous.',
  client_requis: "Le serveur refuse ce client (signature d'application absente).",
  introuvable: 'Élément introuvable (déjà retiré ?).',
  deja_imprime: 'Déjà imprimé.',
  jeton_push_invalide: 'Jeton de notification invalide.',
  appareil_inconnu: "Ce téléphone n'est pas inscrit aux notifications.",
  envoi_push_echoue: "La notification n'est pas partie (service push d'Expo).",
};

/** Message lisible pour l'utilisateur. */
export function messageErreur(e: unknown): string {
  if (e instanceof ErreurApi) {
    if (e.reseau) return MESSAGE_RESEAU;
    const m = MESSAGES[e.code];
    if (m) return e.detail && e.code === 'envoi_push_echoue' ? `${m} ${e.detail}` : m;
    return e.detail || e.code;
  }
  return e instanceof Error ? e.message : String(e);
}

// ---------- Configuration et stockage ----------

export const VERSION_APP: string = Constants.expoConfig?.version ?? '0.0.0';
const USER_AGENT = `EKO-RIP-Client/mobile-${VERSION_APP}`;

/** Ajoute le schéma et retire le « / » final ; un chemin éventuel (https://hote/s/atelier) est conservé. */
export function normaliserUrl(brut: string): string {
  let u = brut.trim();
  if (!u) return '';
  if (!/^https?:\/\//i.test(u)) u = `http://${u}`;
  return u.replace(/\/+$/, '');
}

export const stockage = {
  async lire(cle: string): Promise<string | null> {
    if (modeDemo) return null; // la démo ne lit ni n'écrit rien sur l'appareil
    try { return await SecureStore.getItemAsync(cle); } catch { return null; }
  },
  async ecrire(cle: string, valeur: string | null): Promise<void> {
    if (modeDemo) return;
    try {
      if (valeur === null) await SecureStore.deleteItemAsync(cle);
      else await SecureStore.setItemAsync(cle, valeur);
    } catch { /* stockage indisponible : on continue sans */ }
  },
};

let urlServeur = '';
let modeDemo = false;
let surSessionPerdue: (() => void) | null = null;

/** Mode démo : les requêtes sont servies par des données fictives en mémoire (voir demo.ts). */
export function definirDemo(actif: boolean) {
  modeDemo = actif;
  if (actif) reinitialiserDemo();
}

export function definirServeur(url: string) { urlServeur = normaliserUrl(url); }
export function definirSurSessionPerdue(f: (() => void) | null) { surSessionPerdue = f; }

// ---------- Requêtes ----------

const DELAI_MS = 10000;

/** L'URL est toujours base + chemin par concaténation (jamais new URL(chemin, base), qui écraserait le chemin de base). */
async function brut(methode: string, chemin: string, corps?: unknown): Promise<Response> {
  const ctrl = new AbortController();
  const minuteur = setTimeout(() => ctrl.abort(), DELAI_MS);
  try {
    return await fetch(`${urlServeur}${chemin}`, {
      method: methode,
      credentials: auth.credentials ?? 'omit',
      signal: ctrl.signal,
      headers: {
        ...(await auth.entetes()),
        'User-Agent': USER_AGENT,
        Accept: 'application/json',
        ...(corps !== undefined ? { 'Content-Type': 'application/json' } : {}),
      },
      body: corps !== undefined ? JSON.stringify(corps) : undefined,
    });
  } catch {
    throw new ErreurApi(0, 'reseau', MESSAGE_RESEAU);
  } finally {
    clearTimeout(minuteur);
  }
}

async function lireErreur(r: Response): Promise<ErreurApi> {
  let code = `http_${r.status}`;
  let detail = `Erreur du serveur (${r.status})`;
  try {
    const j = await r.json();
    if (j?.code) code = j.code;
    if (j?.detail) detail = j.detail;
  } catch { /* corps non JSON */ }
  return new ErreurApi(r.status, code, detail);
}

async function connecter(nom: string, motDePasse: string): Promise<void> {
  const r = await brut('POST', '/api/session', { nom, mot_de_passe: motDePasse });
  if (!r.ok) throw await lireErreur(r);
}

// ---------- Authentification (interchangeable) ----------

/** Authentification branchable : cookie de session local aujourd'hui, Bearer OAuth plus tard. */
export interface Auth {
  /** En-têtes d'authentification ajoutés à chaque requête. */
  entetes(): Promise<Record<string, string>>;
  /** Appelé sur 401 `connexion_requise` ; vrai si la requête peut être rejouée. */
  renouveler(): Promise<boolean>;
  /** Mode d'envoi des cookies de fetch (défaut : 'omit'). */
  credentials?: RequestCredentials;
}

/** Cookie `eko_rip_session` (stockage natif) + reconnexion avec les identifiants du SecureStore. */
export class AuthCookieLocal implements Auth {
  credentials: RequestCredentials = 'include';
  // Une seule reconnexion à la fois, même si plusieurs requêtes échouent ensemble
  private reconnexion: Promise<boolean> | null = null;

  async entetes() { return {}; }

  renouveler(): Promise<boolean> {
    this.reconnexion ??= (async () => {
      try {
        const nom = await stockage.lire('nom');
        const mdp = await stockage.lire('mdp');
        if (!nom || !mdp) return false;
        await connecter(nom, mdp);
        return true;
      } catch {
        return false;
      } finally {
        this.reconnexion = null;
      }
    })();
    return this.reconnexion;
  }
}

let auth: Auth = new AuthCookieLocal();
export function definirAuth(a: Auth) { auth = a; }

async function requete<T>(methode: string, chemin: string, corps?: unknown, retenter = true): Promise<T> {
  if (modeDemo) return reponseDemo<T>(methode, chemin);
  const r = await brut(methode, chemin, corps);
  if (r.ok) {
    if (r.status === 204) return undefined as T;
    return (await r.json()) as T;
  }
  const err = await lireErreur(r);
  if (r.status === 401 && err.code === 'connexion_requise' && retenter && chemin !== '/api/session') {
    if (await auth.renouveler()) return requete<T>(methode, chemin, corps, false);
    surSessionPerdue?.();
  }
  throw err;
}

// ---------- Points d'accès ----------

export const api = {
  session: () => requete<InfoSession>('GET', '/api/session'),
  connexion: async (nom: string, mdp: string) => { await connecter(nom, mdp); },
  deconnexion: () => requete<void>('DELETE', '/api/session', undefined, false),

  file: () => requete<Sortie[]>('GET', '/api/file'),
  action: (id: string, action: ActionSortie) =>
    requete<void>('POST', `/api/file/${encodeURIComponent(id)}/${action}`),

  etat: () => requete<EtatMobile>('GET', '/api/mobile/etat'),
  evenements: (depuis = 0, limite = 50) =>
    requete<{ evenements: Evenement[]; dernier: number }>(
      'GET', `/api/mobile/evenements?depuis=${depuis}&limite=${limite}`),

  enregistrerAppareil: (jeton: string, nom: string, plateforme: 'ios' | 'android', fins: boolean) =>
    requete<void>('POST', '/api/mobile/appareils', { jeton, nom, plateforme, fins }),
  retirerAppareil: (jeton: string) =>
    requete<void>('POST', '/api/mobile/appareils/retirer', { jeton }),
  essaiNotification: (jeton: string) =>
    requete<void>('POST', '/api/mobile/essai', { jeton }),
};

/** Teste une URL de serveur sans se connecter (utilisé par l'écran de connexion). */
export async function sonderServeur(url: string): Promise<InfoSession> {
  definirServeur(url);
  return api.session();
}

/** Les événements « machine » ouvrent l'onglet Machines, les autres l'onglet Alertes. */
export function estEvenementMachine(g: GenreEvenement): boolean {
  return ['alerte_machine', 'encre_basse', 'encre_vide', 'machine_injoignable', 'machine_revenue'].includes(g);
}
