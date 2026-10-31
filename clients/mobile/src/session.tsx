import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { Platform } from 'react-native';
import {
  api, definirDemo, definirServeur, definirSurSessionPerdue, ErreurApi, InfoSession, normaliserUrl, sonderServeur, stockage,
} from './api';
import {
  activerNotifications, desactiverNotifications, ecrirePrefs, ETAT_NOTIF_INITIAL, EtatNotif, lirePrefs, PrefsNotif,
} from './notifications';

type Statut = 'chargement' | 'erreur_reseau' | 'deconnecte' | 'connecte';

interface Session {
  statut: Statut;
  url: string;
  utilisateur: string | null;
  role: InfoSession['role'];
  accesLibre: boolean;
  notif: EtatNotif;
  prefs: PrefsNotif;
  /** Mode démo : données fictives, aucun serveur ni notification. */
  demo: boolean;
  /** Vérifie le serveur ; renvoie l'info de session (lève ErreurApi si injoignable). */
  sonder(url: string): Promise<InfoSession>;
  /** Se connecte (nom / mdp ignorés en accès libre). */
  connexion(url: string, nom?: string, mdp?: string): Promise<void>;
  deconnexion(): Promise<void>;
  demarrerDemo(): void;
  quitterDemo(): void;
  reessayer(): void;
  reglerNotifications(p: Partial<PrefsNotif>): Promise<void>;
}

const Ctx = createContext<Session | null>(null);

export function useSession(): Session {
  const s = useContext(Ctx);
  if (!s) throw new Error('SessionProvider manquant');
  return s;
}

export function SessionProvider({ children }: { children: React.ReactNode }) {
  const [statut, setStatut] = useState<Statut>('chargement');
  const [url, setUrl] = useState('');
  const [info, setInfo] = useState<InfoSession>({ acces_libre: false, utilisateur: null, role: null });
  const [notif, setNotif] = useState<EtatNotif>(ETAT_NOTIF_INITIAL);
  const [prefs, setPrefs] = useState<PrefsNotif>({ actives: true, fins: false });
  const prefsRef = useRef(prefs);
  prefsRef.current = prefs;
  const [demo, setDemo] = useState(false);
  const jetonRef = useRef<string | null>(null);

  // Enregistre l'appareil pour les notifications push si l'utilisateur les veut
  const synchroniserNotifs = useCallback(async (p: PrefsNotif) => {
    if (!p.actives) {
      await desactiverNotifications(jetonRef.current);
      jetonRef.current = null;
      setNotif({ code: 'desactive', message: 'Notifications désactivées.', jeton: null });
      return;
    }
    const etat = await activerNotifications(p.fins);
    jetonRef.current = etat.jeton;
    setNotif(etat);
  }, []);

  const finConnexion = useCallback(async (i: InfoSession) => {
    setInfo(i);
    setStatut('connecte');
    const p = await lirePrefs();
    setPrefs(p);
    synchroniserNotifs(p);
  }, [synchroniserNotifs]);

  const demarrerDemo = useCallback(() => {
    definirDemo(true);
    setDemo(true);
    setInfo({ acces_libre: true, utilisateur: null, role: 'admin' });
    setNotif({ code: 'desactive', message: 'Notifications indisponibles en mode démo.', jeton: null });
    setStatut('connecte');
  }, []);

  const quitterDemo = useCallback(() => {
    definirDemo(false);
    setDemo(false);
    setInfo({ acces_libre: false, utilisateur: null, role: null });
    setNotif(ETAT_NOTIF_INITIAL);
    setStatut('deconnecte');
  }, []);

  // Démarrage : reprise de la session avec l'URL et les identifiants mémorisés
  const demarrer = useCallback(async () => {
    // Sur le web (captures d'écran), ?demo=1 ouvre directement le mode démo
    if (Platform.OS === 'web' && globalThis.location?.search.includes('demo=1')) return demarrerDemo();
    setStatut('chargement');
    const urlMemo = await stockage.lire('url');
    if (!urlMemo) return setStatut('deconnecte');
    setUrl(urlMemo);
    definirServeur(urlMemo);
    try {
      let i = await api.session();
      if (!i.acces_libre && !i.utilisateur) {
        const nom = await stockage.lire('nom');
        const mdp = await stockage.lire('mdp');
        if (!nom || !mdp) return setStatut('deconnecte');
        await api.connexion(nom, mdp);
        i = await api.session();
      }
      await finConnexion(i);
    } catch (e) {
      if (e instanceof ErreurApi && e.reseau) setStatut('erreur_reseau');
      else setStatut('deconnecte');
    }
  }, [finConnexion, demarrerDemo]);

  useEffect(() => { demarrer(); }, [demarrer]);

  // Une requête n'a pas pu se reconnecter : retour à l'écran de connexion
  useEffect(() => {
    definirSurSessionPerdue(() => setStatut((s) => (s === 'connecte' ? 'deconnecte' : s)));
    return () => definirSurSessionPerdue(null);
  }, []);

  const valeur = useMemo<Session>(() => ({
    statut,
    url,
    utilisateur: info.utilisateur,
    role: info.role,
    accesLibre: info.acces_libre,
    notif,
    prefs,
    demo,
    sonder: (u) => sonderServeur(u),
    async connexion(u, nom, mdp) {
      const base = normaliserUrl(u);
      let i = await sonderServeur(base);
      if (!i.acces_libre) {
        if (!nom || !mdp) throw new ErreurApi(401, 'identifiants_invalides', 'Nom et mot de passe requis.');
        await api.connexion(nom, mdp);
        i = await api.session();
        await stockage.ecrire('nom', nom);
        await stockage.ecrire('mdp', mdp);
      } else {
        await stockage.ecrire('nom', null);
        await stockage.ecrire('mdp', null);
      }
      await stockage.ecrire('url', base);
      setUrl(base);
      await finConnexion(i);
    },
    async deconnexion() {
      const jeton = jetonRef.current;
      jetonRef.current = null;
      await stockage.ecrire('nom', null);
      await stockage.ecrire('mdp', null);
      setInfo({ acces_libre: false, utilisateur: null, role: null });
      setNotif(ETAT_NOTIF_INITIAL);
      setStatut('deconnecte');
      // Côté serveur, sans bloquer l'interface (le serveur peut être injoignable)
      desactiverNotifications(jeton)
        .then(() => api.deconnexion())
        .catch(() => {});
    },
    demarrerDemo,
    quitterDemo,
    reessayer: demarrer,
    async reglerNotifications(patch) {
      const p = { ...prefsRef.current, ...patch };
      setPrefs(p);
      if (demo) return;
      await ecrirePrefs(p);
      await synchroniserNotifs(p);
    },
  }), [statut, url, info, notif, prefs, demo, demarrer, demarrerDemo, quitterDemo, finConnexion, synchroniserNotifs]);

  return <Ctx.Provider value={valeur}>{children}</Ctx.Provider>;
}
