import { useFocusEffect } from 'expo-router';
import { useCallback, useEffect, useRef, useState } from 'react';
import { AppState } from 'react-native';
import { messageErreur } from './api';

/** Vrai tant que l'application est au premier plan. */
export function useAppActive(): boolean {
  const [actif, setActif] = useState(AppState.currentState === 'active');
  useEffect(() => {
    const abo = AppState.addEventListener('change', (s) => setActif(s === 'active'));
    return () => abo.remove();
  }, []);
  return actif;
}

/**
 * Charge des données, puis les rafraîchit toutes les `ms` millisecondes
 * tant que l'écran est affiché et l'app au premier plan.
 */
export function useDonnees<T>(chargeur: () => Promise<T>, ms: number) {
  const [donnees, setDonnees] = useState<T | null>(null);
  const [erreur, setErreur] = useState<string | null>(null);
  const [actualisation, setActualisation] = useState(false);
  const actif = useAppActive();
  const ref = useRef(chargeur);
  ref.current = chargeur;

  const charger = useCallback(async () => {
    try {
      setDonnees(await ref.current());
      setErreur(null);
    } catch (e) {
      setErreur(messageErreur(e)); // on garde les dernières données affichées
    }
  }, []);

  useFocusEffect(
    useCallback(() => {
      if (!actif) return;
      charger();
      const t = setInterval(charger, ms);
      return () => clearInterval(t);
    }, [actif, charger, ms]),
  );

  // Tirer pour actualiser
  const actualiser = useCallback(async () => {
    setActualisation(true);
    await charger();
    setActualisation(false);
  }, [charger]);

  return { donnees, erreur, actualisation, actualiser, charger };
}
