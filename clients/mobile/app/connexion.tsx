import { useEffect, useState } from 'react';
import {
  ActivityIndicator, KeyboardAvoidingView, Platform, Pressable, ScrollView, StyleSheet, Text, TextInput, View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { ErreurApi, messageErreur, stockage } from '../src/api';
import { useSession } from '../src/session';
import { useTheme } from '../src/theme';
import { BlocErreur, Bouton } from '../src/ui';

export default function Connexion() {
  const { sonder, connexion, demarrerDemo, url: urlMemo } = useSession();
  const t = useTheme();
  const insets = useSafeAreaInsets();
  const [url, setUrl] = useState('');
  const [nom, setNom] = useState('');
  const [mdp, setMdp] = useState('');
  const [identifiants, setIdentifiants] = useState(false); // le serveur exige un compte
  const [occupe, setOccupe] = useState(false);
  const [erreur, setErreur] = useState<string | null>(null);

  // Pré-remplit avec le dernier serveur utilisé
  useEffect(() => {
    if (urlMemo) setUrl(urlMemo);
    else stockage.lire('url').then((u) => u && setUrl(u));
  }, [urlMemo]);

  async function valider() {
    setErreur(null);
    setOccupe(true);
    try {
      if (!identifiants) {
        const info = await sonder(url);
        if (info.acces_libre) await connexion(url);
        else setIdentifiants(true);
      } else {
        await connexion(url, nom.trim(), mdp);
      }
    } catch (e) {
      if (e instanceof ErreurApi && e.code === 'identifiants_invalides') setErreur('Nom ou mot de passe incorrect.');
      else if (e instanceof ErreurApi && e.code === 'trop_d_echecs') setErreur('Trop de tentatives échouées. Réessayez dans quelques minutes.');
      else setErreur(messageErreur(e));
    } finally {
      setOccupe(false);
    }
  }

  const champ = [styles.champ, { backgroundColor: t.carte, color: t.texte }];

  return (
    <KeyboardAvoidingView style={{ flex: 1 }} behavior={Platform.OS === 'ios' ? 'padding' : undefined}>
      <ScrollView
        keyboardShouldPersistTaps="handled"
        contentContainerStyle={[styles.contenu, { paddingTop: insets.top + 48, paddingBottom: insets.bottom + 24 }]}>
        <Text style={[styles.titre, { color: t.texte }]}>E-KO RIP</Text>
        <Text style={[styles.sousTitre, { color: t.texteDiscret }]}>
          Suivi de la file d'impression et des machines
        </Text>

        <Text style={[styles.etiquette, { color: t.texteDiscret }]}>Serveur</Text>
        <TextInput
          style={champ}
          value={url}
          onChangeText={(v) => { setUrl(v); setIdentifiants(false); }}
          placeholder="http://192.168.10.109:8640"
          placeholderTextColor={t.texteDiscret}
          autoCapitalize="none"
          autoCorrect={false}
          keyboardType="url"
          editable={!occupe}
        />

        {identifiants && (
          <>
            <Text style={[styles.etiquette, { color: t.texteDiscret }]}>Compte</Text>
            <TextInput
              style={champ}
              value={nom}
              onChangeText={setNom}
              placeholder="Nom d'utilisateur"
              placeholderTextColor={t.texteDiscret}
              autoCapitalize="none"
              autoCorrect={false}
              textContentType="username"
              editable={!occupe}
            />
            <TextInput
              style={champ}
              value={mdp}
              onChangeText={setMdp}
              placeholder="Mot de passe"
              placeholderTextColor={t.texteDiscret}
              secureTextEntry
              textContentType="password"
              editable={!occupe}
              onSubmitEditing={valider}
            />
          </>
        )}

        {erreur && <View style={{ marginHorizontal: -16 }}><BlocErreur message={erreur} /></View>}

        <View style={{ marginTop: 16 }}>
          {occupe ? (
            <ActivityIndicator style={{ padding: 14 }} />
          ) : (
            <Bouton
              titre={identifiants ? 'Se connecter' : 'Continuer'}
              onPress={valider}
              desactive={!url.trim() || (identifiants && (!nom.trim() || !mdp))}
            />
          )}
        </View>

        <Pressable onPress={demarrerDemo} disabled={occupe} hitSlop={8} style={styles.demo}>
          <Text style={{ color: t.accent, fontSize: 15 }}>Essayer le mode démo</Text>
        </Pressable>
      </ScrollView>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  contenu: { paddingHorizontal: 24 },
  titre: { fontSize: 34, fontWeight: '700' },
  sousTitre: { fontSize: 15, marginTop: 4, marginBottom: 32 },
  etiquette: { fontSize: 13, textTransform: 'uppercase', marginBottom: 6, marginTop: 12 },
  demo: { alignSelf: 'center', marginTop: 28, padding: 6 },
  champ: { borderRadius: 12, paddingHorizontal: 14, paddingVertical: 13, fontSize: 17, marginBottom: 8 },
});
