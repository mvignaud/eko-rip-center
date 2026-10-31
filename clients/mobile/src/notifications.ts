import Constants from 'expo-constants';
import * as Device from 'expo-device';
import * as Notifications from 'expo-notifications';
import { Platform } from 'react-native';
import { api, messageErreur, stockage } from './api';

const PROJET_PLACEHOLDER = 'A_RENSEIGNER_PAR_EAS_INIT';

export interface EtatNotif {
  code: 'inactif' | 'ok' | 'desactive' | 'refuse' | 'simulateur' | 'projet' | 'erreur';
  message: string;
  jeton: string | null;
}

export const ETAT_NOTIF_INITIAL: EtatNotif = { code: 'inactif', message: '', jeton: null };

/** Bannière + son même quand l'app est au premier plan, et canal Android « alertes ». */
export function configurerNotifications() {
  Notifications.setNotificationHandler({
    handleNotification: async () => ({
      shouldShowBanner: true,
      shouldShowList: true,
      shouldPlaySound: true,
      shouldSetBadge: false,
    }),
  });
  if (Platform.OS === 'android') {
    Notifications.setNotificationChannelAsync('alertes', {
      name: 'Alertes machines',
      importance: Notifications.AndroidImportance.HIGH,
      vibrationPattern: [0, 250, 250, 250],
      lightColor: '#0A84FF',
    }).catch(() => {});
  }
}

// ---------- Préférences (stockées sur l'appareil) ----------

export interface PrefsNotif { actives: boolean; fins: boolean }

export async function lirePrefs(): Promise<PrefsNotif> {
  return {
    actives: (await stockage.lire('notif_actives')) !== '0',
    fins: (await stockage.lire('notif_fins')) === '1',
  };
}

export async function ecrirePrefs(p: PrefsNotif) {
  await stockage.ecrire('notif_actives', p.actives ? '1' : '0');
  await stockage.ecrire('notif_fins', p.fins ? '1' : '0');
}

// ---------- Enregistrement ----------

/** Demande l'autorisation, récupère le jeton Expo et l'enregistre sur le serveur RIP. */
export async function activerNotifications(fins: boolean): Promise<EtatNotif> {
  if (!Device.isDevice) {
    return { code: 'simulateur', message: 'Notifications : indisponibles sur simulateur.', jeton: null };
  }
  const projectId: string | undefined = Constants.expoConfig?.extra?.eas?.projectId;
  if (!projectId || projectId === PROJET_PLACEHOLDER) {
    return { code: 'projet', message: 'Notifications : projet EAS non configuré', jeton: null };
  }
  try {
    let { status } = await Notifications.getPermissionsAsync();
    if (status !== 'granted') ({ status } = await Notifications.requestPermissionsAsync());
    if (status !== 'granted') {
      return {
        code: 'refuse',
        message: "Notifications refusées : autorisez-les dans les réglages du téléphone.",
        jeton: null,
      };
    }
    const { data: jeton } = await Notifications.getExpoPushTokenAsync({ projectId });
    const nom = Device.deviceName ?? Device.modelName ?? 'Appareil';
    await api.enregistrerAppareil(jeton, nom, Platform.OS === 'ios' ? 'ios' : 'android', fins);
    await stockage.ecrire('jeton', jeton);
    return { code: 'ok', message: 'Notifications activées.', jeton };
  } catch (e) {
    return { code: 'erreur', message: `Notifications : ${messageErreur(e)}`, jeton: null };
  }
}

/** Retire l'appareil du serveur (déconnexion ou notifications coupées). */
export async function desactiverNotifications(jeton: string | null): Promise<void> {
  const j = jeton ?? (await stockage.lire('jeton'));
  if (!j) return;
  try { await api.retirerAppareil(j); } catch { /* serveur absent : sans importance */ }
  await stockage.ecrire('jeton', null);
}
