import * as Notifications from 'expo-notifications';
import { Platform } from 'react-native';
import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { api, cleServeur, Evenement, messageErreur, stockage } from './api';
import { useAppActive } from './hooks';

interface Alertes {
  evenements: Evenement[];
  erreur: string | null;
  /** Nombre d'événements pas encore vus. */
  nonVus: number;
  /** Dernier id vu (avant le dernier marquage). */
  vu: number;
  rafraichir(): Promise<void>;
  marquerVus(): void;
}

const Ctx = createContext<Alertes | null>(null);

export function useAlertes(): Alertes {
  const a = useContext(Ctx);
  if (!a) throw new Error('AlertesProvider manquant');
  return a;
}

const INTERVALLE_MS = 20000;

/** Interroge les événements en continu (app active) pour alimenter la pastille de l'onglet. */
export function AlertesProvider({ children }: { children: React.ReactNode }) {
  const [evenements, setEvenements] = useState<Evenement[]>([]);
  const [erreur, setErreur] = useState<string | null>(null);
  const [vu, setVu] = useState<number | null>(null);
  const vuRef = useRef<number | null>(null);
  const actif = useAppActive();

  const definirVu = useCallback((n: number) => {
    vuRef.current = n;
    setVu(n);
    // Numéros d'événements propres à chaque serveur : un curseur par serveur
    stockage.ecrire(cleServeur('dernier_vu'), String(n));
  }, []);

  const rafraichir = useCallback(async () => {
    try {
      const r = await api.evenements(0, 50);
      setEvenements(r.evenements);
      setErreur(null);
      if (vuRef.current === null) {
        const memo = await stockage.lire(cleServeur('dernier_vu'));
        if (memo !== null) { vuRef.current = Number(memo); setVu(Number(memo)); }
        else definirVu(r.dernier); // premier lancement : l'historique n'est pas « nouveau »
      }
    } catch (e) {
      setErreur(messageErreur(e));
    }
  }, [definirVu]);

  useEffect(() => {
    if (!actif) return;
    rafraichir();
    const t = setInterval(rafraichir, INTERVALLE_MS);
    return () => clearInterval(t);
  }, [actif, rafraichir]);

  // Notification reçue au premier plan : on recharge tout de suite
  useEffect(() => {
    if (Platform.OS === 'web') return;
    const abo = Notifications.addNotificationReceivedListener(() => { rafraichir(); });
    return () => abo.remove();
  }, [rafraichir]);

  const valeur = useMemo<Alertes>(() => ({
    evenements,
    erreur,
    vu: vu ?? 0,
    nonVus: vu === null ? 0 : evenements.filter((e) => e.id > vu).length,
    rafraichir,
    marquerVus() {
      const max = evenements.reduce((m, e) => Math.max(m, e.id), 0);
      if (max > (vuRef.current ?? 0)) definirVu(max);
    },
  }), [evenements, erreur, vu, rafraichir, definirVu]);

  return <Ctx.Provider value={valeur}>{children}</Ctx.Provider>;
}
