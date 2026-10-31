import { router, Stack } from 'expo-router';
import { StatusBar } from 'expo-status-bar';
import { useEffect, useRef } from 'react';
import { ActivityIndicator, StyleSheet, Text, View } from 'react-native';
import { estEvenementMachine, Evenement, MESSAGE_RESEAU } from '../src/api';
import { configurerNotifications, useReponseNotification } from '../src/notifications';
import { SessionProvider, useSession } from '../src/session';
import { useTheme } from '../src/theme';
import { Bouton } from '../src/ui';

configurerNotifications();

export default function RootLayout() {
  return (
    <SessionProvider>
      <StatusBar style="auto" />
      <Racine />
    </SessionProvider>
  );
}

function Racine() {
  const { statut, reessayer, deconnexion } = useSession();
  const t = useTheme();
  const traitee = useRef<string | null>(null);

  // Toucher une notification ouvre l'onglet concerné (aussi au démarrage à froid)
  const reponse = useReponseNotification();
  useEffect(() => {
    if (!reponse || statut !== 'connecte') return;
    const id = reponse.notification.request.identifier;
    if (traitee.current === id) return;
    traitee.current = id;
    const ev = reponse.notification.request.content.data?.evenement as Evenement | undefined;
    router.navigate(ev && estEvenementMachine(ev.genre) ? '/machines' : '/alertes');
  }, [reponse, statut]);

  return (
    <View style={{ flex: 1, backgroundColor: t.fond }}>
      <Stack screenOptions={{ headerShown: false }}>
        <Stack.Protected guard={statut === 'connecte'}>
          <Stack.Screen name="(tabs)" />
        </Stack.Protected>
        <Stack.Protected guard={statut !== 'connecte'}>
          <Stack.Screen name="connexion" />
        </Stack.Protected>
      </Stack>
      {(statut === 'chargement' || statut === 'erreur_reseau') && (
        <View style={[StyleSheet.absoluteFill, styles.centre, { backgroundColor: t.fond }]}>
          {statut === 'chargement' ? (
            <ActivityIndicator size="large" />
          ) : (
            <>
              <Text style={[styles.message, { color: t.texte }]}>{MESSAGE_RESEAU}</Text>
              <View style={styles.boutons}>
                <Bouton titre="Réessayer" onPress={reessayer} />
                <View style={{ height: 10 }} />
                <Bouton titre="Changer de serveur" onPress={deconnexion} couleur={t.texteDiscret} />
              </View>
            </>
          )}
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  centre: { alignItems: 'center', justifyContent: 'center', padding: 24 },
  message: { fontSize: 17, textAlign: 'center', marginBottom: 24 },
  boutons: { alignSelf: 'stretch' },
});
