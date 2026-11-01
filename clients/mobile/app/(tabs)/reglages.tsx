import * as WebBrowser from 'expo-web-browser';
import { useState } from 'react';
import { Alert, Pressable, ScrollView, StyleSheet, Switch, Text, View } from 'react-native';
import { api, messageErreur, VERSION_APP } from '../../src/api';
import { useSession } from '../../src/session';
import { useTheme } from '../../src/theme';
import { Bouton, Carte } from '../../src/ui';

export default function Reglages() {
  const t = useTheme();
  const s = useSession();
  const [occupe, setOccupe] = useState(false);

  async function essai() {
    if (!s.notif.jeton) return;
    setOccupe(true);
    try {
      await api.essaiNotification(s.notif.jeton);
      Alert.alert('Notification d\'essai', 'Demande envoyée : la notification devrait arriver dans quelques secondes.');
    } catch (e) {
      Alert.alert('Échec', messageErreur(e));
    }
    setOccupe(false);
  }

  function quitter() {
    Alert.alert('Déconnexion', 'Se déconnecter du serveur ?', [
      { text: 'Annuler', style: 'cancel' },
      { text: 'Se déconnecter', style: 'destructive', onPress: () => s.deconnexion() },
    ]);
  }

  const ligne = (titre: string, valeur: string) => (
    <View style={styles.ligne}>
      <Text style={{ color: t.texte, fontSize: 16 }}>{titre}</Text>
      <Text style={{ color: t.texteDiscret, fontSize: 16, flexShrink: 1, textAlign: 'right' }}>{valeur}</Text>
    </View>
  );

  return (
    <ScrollView contentContainerStyle={{ paddingVertical: 12 }}>
      <Text style={[styles.titre, { color: t.texteDiscret }]}>Serveur</Text>
      <Carte>
        {ligne('Adresse', s.demo ? 'Mode démo' : s.url)}
        {ligne('Utilisateur', s.demo ? 'Données fictives' : s.accesLibre ? 'Accès libre' : s.utilisateur ?? '—')}
        {!s.accesLibre && ligne('Rôle', s.role === 'admin' ? 'Administrateur' : s.role === 'operateur' ? 'Opérateur' : '—')}
      </Carte>

      <Text style={[styles.titre, { color: t.texteDiscret }]}>Notifications</Text>
      <Carte>
        <View style={styles.ligne}>
          <Text style={{ color: t.texte, fontSize: 16 }}>Notifications</Text>
          <Switch value={s.prefs.actives} onValueChange={(v) => s.reglerNotifications({ actives: v })} />
        </View>
        <View style={styles.ligne}>
          <Text style={{ color: t.texte, fontSize: 16 }}>Erreurs machine</Text>
          <Switch value={s.prefs.actives} disabled />
        </View>
        <View style={styles.ligne}>
          <Text style={{ color: t.texte, fontSize: 16 }}>Fins d'impression</Text>
          <Switch
            value={s.prefs.fins}
            disabled={!s.prefs.actives}
            onValueChange={(v) => s.reglerNotifications({ fins: v })}
          />
        </View>
        {s.notif.message ? (
          <Text style={{ color: s.notif.code === 'ok' ? t.ok : t.texteDiscret, fontSize: 13, marginTop: 6 }}>
            {s.notif.message}
          </Text>
        ) : null}
        <View style={{ marginTop: 12 }}>
          <Bouton
            titre="Envoyer une notification d'essai"
            onPress={essai}
            desactive={!s.notif.jeton || occupe}
          />
        </View>
      </Carte>

      <View style={{ marginHorizontal: 16, marginBottom: 12 }}>
        {s.demo
          ? <Bouton titre="Quitter le mode démo" onPress={s.quitterDemo} couleur={t.erreur} />
          : <Bouton titre="Se déconnecter" onPress={quitter} couleur={t.erreur} />}
      </View>

      <Pressable onPress={() => WebBrowser.openBrowserAsync(adresseDocumentation())} hitSlop={8} style={styles.aide}>
        <Text style={{ color: t.accent, fontSize: 15 }}>Aide / Documentation</Text>
      </Pressable>

      <Text style={[styles.version, { color: t.texteDiscret }]}>E-KO RIP version {VERSION_APP}</Text>
    </ScrollView>
  );
}

/** Page de l'app dans la documentation publique, en français ou en anglais selon le téléphone. */
function adresseDocumentation(): string {
  let langue = 'fr';
  try { langue = Intl.DateTimeFormat().resolvedOptions().locale.slice(0, 2).toLowerCase(); } catch { /* fr */ }
  return langue === 'fr'
    ? 'https://e-ko.rip/documentation/application-mobile/'
    : 'https://e-ko.rip/documentation/en/mobile-app/';
}

const styles = StyleSheet.create({
  titre: { fontSize: 13, fontWeight: '600', textTransform: 'uppercase', marginHorizontal: 20, marginTop: 12, marginBottom: 8 },
  ligne: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', paddingVertical: 8, gap: 12 },
  aide: { alignSelf: 'center', padding: 6, marginBottom: 4 },
  version: { textAlign: 'center', fontSize: 13, marginTop: 8 },
});
