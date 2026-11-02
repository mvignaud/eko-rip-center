import { useFocusEffect } from 'expo-router';
import { useCallback, useEffect, useRef, useState } from 'react';
import { FlatList, RefreshControl, StyleSheet, Text, View } from 'react-native';
import { Evenement } from '../../src/api';
import { useAlertes } from '../../src/alertes';
import { ilYa } from '../../src/format';
import { useTheme } from '../../src/theme';
import { BlocErreur, Carte, Vide } from '../../src/ui';

export default function Alertes() {
  const t = useTheme();
  const { evenements, erreur, rafraichir, marquerVus, vu } = useAlertes();
  const [actualisation, setActualisation] = useState(false);
  const [seuil, setSeuil] = useState(vu); // les événements au-dessus sont « nouveaux »
  const vuRef = useRef(vu);
  vuRef.current = vu;

  // À l'ouverture : on retient ce qui était nouveau, puis on marque tout comme vu
  useFocusEffect(useCallback(() => {
    setSeuil(vuRef.current);
    return undefined;
  }, []));
  useFocusEffect(useCallback(() => { marquerVus(); }, [marquerVus]));
  useEffect(() => { if (vu < seuil) setSeuil(vu); }, [vu, seuil]);

  const couleur = (e: Evenement) =>
    e.gravite === 'erreur' ? t.erreur : e.gravite === 'alerte' ? t.alerte : t.info;

  return (
    <FlatList
      data={evenements}
      keyExtractor={(e) => String(e.id)}
      contentContainerStyle={{ paddingVertical: 12 }}
      refreshControl={
        <RefreshControl
          refreshing={actualisation}
          onRefresh={async () => { setActualisation(true); await rafraichir(); setActualisation(false); }}
        />
      }
      ListHeaderComponent={erreur ? <BlocErreur message={erreur} onRetry={rafraichir} /> : undefined}
      ListEmptyComponent={erreur ? undefined : <Vide texte="Aucune alerte pour le moment." />}
      renderItem={({ item: e }) => (
        <Carte style={{ marginBottom: 8, borderLeftWidth: 4, borderLeftColor: couleur(e) }}>
          <View style={styles.haut}>
            <Text style={[styles.titre, { color: couleur(e) }]}>{e.titre}</Text>
            {e.id > seuil && <View style={[styles.point, { backgroundColor: t.accent }]} />}
          </View>
          {e.texte ? <Text style={[styles.texte, { color: t.texte }]}>{e.texte}</Text> : null}
          <Text style={[styles.temps, { color: t.texteDiscret }]}>{ilYa(e.le)}</Text>
        </Carte>
      )}
    />
  );
}

const styles = StyleSheet.create({
  haut: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' },
  titre: { fontSize: 16, fontWeight: '600', flex: 1 },
  point: { width: 10, height: 10, borderRadius: 5, marginLeft: 8 },
  texte: { fontSize: 14, marginTop: 4 },
  temps: { fontSize: 12, marginTop: 6 },
});
