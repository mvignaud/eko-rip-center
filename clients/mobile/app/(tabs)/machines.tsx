import { RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { api, Machine } from '../../src/api';
import { couleurEncre } from '../../src/encres';
import { duree, ilYa, metres } from '../../src/format';
import { useDonnees } from '../../src/hooks';
import { Theme, useTheme } from '../../src/theme';
import { Badge, BlocErreur, Carte, Vide } from '../../src/ui';

function etatMachine(m: Machine, t: Theme): { texte: string; couleur: string } {
  if (m.snmp && m.joignable === false) return { texte: 'Injoignable', couleur: t.erreur };
  if (m.en_cours) {
    const decoupe = !m.operations.includes('impression');
    return { texte: decoupe ? 'En découpe' : 'En impression', couleur: t.ok };
  }
  return { texte: 'En attente', couleur: t.texteDiscret };
}

export default function Machines() {
  const t = useTheme();
  const { donnees, erreur, actualisation, actualiser, charger } = useDonnees(() => api.etat(), 15000);

  return (
    <ScrollView
      contentContainerStyle={{ paddingVertical: 12 }}
      refreshControl={<RefreshControl refreshing={actualisation} onRefresh={actualiser} />}>
      {erreur && <BlocErreur message={erreur} onRetry={charger} />}
      {!donnees && !erreur && <Vide texte="Chargement…" />}
      {donnees?.machines.length === 0 && <Vide texte="Aucune machine configurée." />}
      {donnees?.machines.map((m) => <CarteMachine key={m.id} m={m} />)}
    </ScrollView>
  );
}

function CarteMachine({ m }: { m: Machine }) {
  const t = useTheme();
  const etat = etatMachine(m, t);
  return (
    <Carte>
      <View style={styles.haut}>
        <View style={{ flex: 1 }}>
          <Text style={[styles.nom, { color: t.texte }]}>{m.nom}</Text>
          <Text style={[styles.detail, { color: t.texteDiscret }]}>{m.modele}</Text>
        </View>
        <Badge texte={etat.texte} couleur={etat.couleur} />
      </View>

      {m.alertes.map((a, i) => (
        <Text key={i} style={[styles.alerte, { color: t.erreur }]}>⚠ {a}</Text>
      ))}

      {m.en_cours && (
        <Text style={[styles.ligne, { color: t.texte }]}>
          En cours : {m.en_cours.nom} (page {m.en_cours.page})
          {m.en_cours.debut ? ` · depuis ${duree(Date.now() / 1000 - m.en_cours.debut)}` : ''}
        </Text>
      )}
      <Text style={[styles.detail, { color: t.texteDiscret, marginTop: 6 }]}>
        {m.en_attente} en attente · {m.suspendues} suspendue{m.suspendues > 1 ? 's' : ''}
      </Text>

      {m.encres.length > 0 && (
        <View style={{ marginTop: 10 }}>
          {m.encres.map((e) => <BarreEncre key={e.encre} code={e.encre} pourcent={e.pourcent} />)}
        </View>
      )}

      {m.longueur_restante_m !== null && (
        <Text style={[styles.ligne, { color: t.texte }]}>Rouleau : {metres(m.longueur_restante_m)} restants</Text>
      )}
      {m.releve_le !== null && (
        <Text style={[styles.detail, { color: t.texteDiscret, marginTop: 6 }]}>Relevé {ilYa(m.releve_le)}</Text>
      )}
    </Carte>
  );
}

function BarreEncre({ code, pourcent }: { code: string; pourcent: number }) {
  const t = useTheme();
  const p = Math.max(0, Math.min(100, pourcent));
  // rouge ≤ 5 %, orange ≤ 15 %, sinon la couleur de l'encre
  const couleur = p <= 5 ? t.erreur : p <= 15 ? t.alerte : couleurEncre(code);
  return (
    <View style={styles.encre}>
      <View style={[styles.pastille, { backgroundColor: couleurEncre(code), borderColor: t.separateur }]} />
      <Text style={[styles.code, { color: t.texte }]}>{code}</Text>
      <View style={[styles.piste, { backgroundColor: t.piste }]}>
        <View style={{ width: `${p}%`, height: '100%', backgroundColor: couleur, borderRadius: 4 }} />
      </View>
      <Text style={[styles.pct, { color: p <= 15 ? couleur : t.texteDiscret }]}>{Math.round(pourcent)} %</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  haut: { flexDirection: 'row', alignItems: 'flex-start', gap: 8 },
  nom: { fontSize: 18, fontWeight: '700' },
  detail: { fontSize: 13 },
  ligne: { fontSize: 15, marginTop: 8 },
  alerte: { fontSize: 14, fontWeight: '600', marginTop: 8 },
  encre: { flexDirection: 'row', alignItems: 'center', marginVertical: 3 },
  pastille: { width: 10, height: 10, borderRadius: 5, borderWidth: StyleSheet.hairlineWidth, marginRight: 6 },
  code: { width: 28, fontSize: 13, fontWeight: '600' },
  piste: { flex: 1, height: 8, borderRadius: 4, overflow: 'hidden' },
  pct: { width: 48, textAlign: 'right', fontSize: 13 },
});
