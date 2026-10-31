import { useMemo, useState } from 'react';
import { Alert, Modal, Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { ActionSortie, api, EtatSortie, messageErreur, Sortie } from '../../src/api';
import { duree } from '../../src/format';
import { useDonnees } from '../../src/hooks';
import { Theme, useTheme } from '../../src/theme';
import { Badge, BlocErreur, Carte, Vide } from '../../src/ui';

const ACTIFS: EtatSortie[] = ['en_cours', 'en_attente', 'suspendue', 'imbriquee'];
const TERMINES: EtatSortie[] = ['terminee', 'echec', 'annulee'];

function etatInfo(etat: EtatSortie, t: Theme): { texte: string; couleur: string } {
  switch (etat) {
    case 'en_cours': return { texte: 'En cours', couleur: t.ok };
    case 'en_attente': return { texte: 'En attente', couleur: t.accent };
    case 'suspendue': return { texte: 'Suspendue', couleur: t.alerte };
    case 'terminee': return { texte: 'Terminée', couleur: t.ok };
    case 'echec': return { texte: 'Échec', couleur: t.erreur };
    case 'annulee': return { texte: 'Annulée', couleur: t.texteDiscret };
    case 'imbriquee': return { texte: 'Imbriquée', couleur: t.texteDiscret };
  }
}

interface Choix { action: ActionSortie; titre: string; destructif?: boolean; confirmation?: string }

// Actions proposées selon l'état de la sortie
function actionsValides(s: Sortie): Choix[] {
  switch (s.etat) {
    case 'suspendue':
      return [
        { action: 'reprendre', titre: s.debut ? 'Reprendre' : 'Lancer' },
        { action: 'en_tete', titre: 'Passer en tête' },
        { action: 'annuler', titre: 'Stop', destructif: true, confirmation: 'Annuler cette sortie ?' },
      ];
    case 'en_attente':
      return [
        { action: 'suspendre', titre: 'Pause' },
        { action: 'en_tete', titre: 'Passer en tête' },
        { action: 'annuler', titre: 'Stop', destructif: true, confirmation: 'Annuler cette sortie ?' },
      ];
    case 'terminee':
    case 'echec':
    case 'annulee':
      return [{ action: 'refaire', titre: 'Relancer', confirmation: 'Créer une nouvelle sortie identique ?' }];
    default:
      return [];
  }
}

export default function File() {
  const t = useTheme();
  const { donnees, erreur, actualisation, actualiser, charger } = useDonnees(
    async () => {
      const [file, etat] = await Promise.all([api.file(), api.etat()]);
      return { file: file.filter((s) => !s.archivee), machines: etat.machines };
    },
    5000,
  );
  const [choisie, setChoisie] = useState<Sortie | null>(null);
  const [recentesOuvert, setRecentesOuvert] = useState(false);

  const { groupes, recentes } = useMemo(() => {
    const file = donnees?.file ?? [];
    const noms = new Map((donnees?.machines ?? []).map((m) => [m.id, m.nom]));
    const ordre = [...noms.keys()];
    for (const s of file) if (!noms.has(s.machine)) { noms.set(s.machine, s.machine); ordre.push(s.machine); }
    const actives = file.filter((s) => ACTIFS.includes(s.etat));
    return {
      groupes: ordre.map((id) => ({
        id,
        nom: noms.get(id)!,
        // en cours d'abord, puis l'ordre de la file
        sorties: actives
          .filter((s) => s.machine === id)
          .sort((a, b) => Number(b.etat === 'en_cours') - Number(a.etat === 'en_cours')),
      })),
      recentes: file
        .filter((s) => TERMINES.includes(s.etat))
        .sort((a, b) => (b.fin ?? b.cree_le) - (a.fin ?? a.cree_le))
        .slice(0, 20),
    };
  }, [donnees]);

  const nomMachine = (id: string) => groupes.find((g) => g.id === id)?.nom ?? id;

  async function lancer(s: Sortie, c: Choix) {
    setChoisie(null);
    const faire = async () => {
      try {
        await api.action(s.id, c.action);
      } catch (e) {
        Alert.alert('Action impossible', messageErreur(e));
      }
      charger();
    };
    if (c.confirmation) {
      Alert.alert(c.titre, `${c.confirmation}\n\n${s.nom} (page ${s.page})`, [
        { text: 'Non', style: 'cancel' },
        { text: c.titre, style: c.destructif ? 'destructive' : 'default', onPress: faire },
      ]);
    } else {
      await faire();
    }
  }

  return (
    <>
      <ScrollView
        contentContainerStyle={{ paddingVertical: 12 }}
        refreshControl={<RefreshControl refreshing={actualisation} onRefresh={actualiser} />}>
        {erreur && <BlocErreur message={erreur} onRetry={charger} />}
        {!donnees && !erreur && <Vide texte="Chargement…" />}

        {groupes.map((g) => (
          <View key={g.id}>
            <Text style={[styles.entete, { color: t.texteDiscret }]}>{g.nom}</Text>
            {g.sorties.length === 0 ? (
              <Text style={[styles.aucune, { color: t.texteDiscret }]}>Aucune sortie en cours ou en attente</Text>
            ) : (
              g.sorties.map((s) => <Ligne key={s.id} s={s} onPress={() => setChoisie(s)} />)
            )}
          </View>
        ))}

        {donnees && (
          <View>
            <Pressable onPress={() => setRecentesOuvert(!recentesOuvert)}>
              <Text style={[styles.entete, { color: t.texteDiscret }]}>
                {recentesOuvert ? '▾' : '▸'} Terminées récemment ({recentes.length})
              </Text>
            </Pressable>
            {recentesOuvert && (recentes.length === 0
              ? <Text style={[styles.aucune, { color: t.texteDiscret }]}>Rien de terminé récemment</Text>
              : recentes.map((s) => (
                <Ligne key={s.id} s={s} machine={nomMachine(s.machine)} onPress={() => setChoisie(s)} />
              )))}
          </View>
        )}
      </ScrollView>

      <FeuilleActions sortie={choisie} onFermer={() => setChoisie(null)} onChoix={lancer} />
    </>
  );
}

function Ligne({ s, machine, onPress }: { s: Sortie; machine?: string; onPress: () => void }) {
  const t = useTheme();
  const info = etatInfo(s.etat, t);
  const maintenant = Date.now() / 1000;
  let temps: string | null = null;
  if (s.etat === 'en_cours' && s.debut) temps = `Démarrée depuis ${duree(maintenant - s.debut)}`
    + (s.duree_min ? ` · durée estimée ${duree(s.duree_min * 60)}` : '');
  else if (s.debut && s.fin) temps = `Durée : ${duree(s.fin - s.debut)}`;
  else if (s.duree_min) temps = `Durée estimée : ${duree(s.duree_min * 60)}`;

  return (
    <Pressable onPress={onPress} style={({ pressed }) => ({ opacity: pressed ? 0.7 : 1 })}>
      <Carte style={{ marginBottom: 8 }}>
        <View style={styles.haut}>
          <Text style={[styles.nom, { color: t.texte }]} numberOfLines={2}>{s.nom}</Text>
          <Badge texte={info.texte} couleur={info.couleur} />
        </View>
        <Text style={[styles.detail, { color: t.texteDiscret }]}>
          {machine ? `${machine} · ` : ''}Page {s.page}{s.operation === 'decoupe' ? ' · découpe' : ''}
          {s.planche_nom ? ` · ${s.planche_nom}` : ''}
        </Text>
        {s.erreur ? <Text style={[styles.detail, { color: t.erreur }]}>{s.erreur}</Text> : null}
        {s.raison_attente ? <Text style={[styles.detail, { color: t.alerte }]}>{s.raison_attente}</Text> : null}
        {temps ? <Text style={[styles.detail, { color: t.texteDiscret }]}>{temps}</Text> : null}
      </Carte>
    </Pressable>
  );
}

function FeuilleActions({ sortie, onFermer, onChoix }: {
  sortie: Sortie | null; onFermer: () => void; onChoix: (s: Sortie, c: Choix) => void;
}) {
  const t = useTheme();
  const insets = useSafeAreaInsets();
  const choix = sortie ? actionsValides(sortie) : [];
  return (
    <Modal visible={!!sortie} transparent animationType="fade" onRequestClose={onFermer}>
      <Pressable style={styles.fond} onPress={onFermer}>
        <Pressable style={[styles.feuille, { backgroundColor: t.carte, paddingBottom: insets.bottom + 12 }]}>
          <Text style={[styles.feuilleTitre, { color: t.texte }]} numberOfLines={2}>{sortie?.nom}</Text>
          <Text style={[styles.detail, { color: t.texteDiscret, marginBottom: 8 }]}>Page {sortie?.page}</Text>
          {sortie?.etat === 'en_cours' && (
            <Text style={[styles.note, { color: t.texteDiscret }]}>
              Cette sortie est déjà en cours : elle ne peut être ni suspendue ni arrêtée depuis l'application.
            </Text>
          )}
          {sortie?.etat === 'imbriquee' && (
            <Text style={[styles.note, { color: t.texteDiscret }]}>Sortie imbriquée dans une planche : aucune action.</Text>
          )}
          {choix.map((c) => (
            <Pressable
              key={c.action}
              onPress={() => sortie && onChoix(sortie, c)}
              style={({ pressed }) => [styles.choix, { borderTopColor: t.separateur, opacity: pressed ? 0.5 : 1 }]}>
              <Text style={{ color: c.destructif ? t.erreur : t.accent, fontSize: 18, textAlign: 'center' }}>
                {c.titre}
              </Text>
            </Pressable>
          ))}
          <Pressable onPress={onFermer} style={[styles.choix, { borderTopColor: t.separateur }]}>
            <Text style={{ color: t.texte, fontSize: 18, textAlign: 'center', fontWeight: '600' }}>Fermer</Text>
          </Pressable>
        </Pressable>
      </Pressable>
    </Modal>
  );
}

const styles = StyleSheet.create({
  entete: { fontSize: 13, fontWeight: '600', textTransform: 'uppercase', marginHorizontal: 20, marginTop: 12, marginBottom: 8 },
  aucune: { marginHorizontal: 20, marginBottom: 8, fontSize: 14 },
  haut: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'flex-start', gap: 8 },
  nom: { flex: 1, fontSize: 16, fontWeight: '600' },
  detail: { fontSize: 13, marginTop: 3 },
  fond: { flex: 1, backgroundColor: '#00000066', justifyContent: 'flex-end' },
  feuille: { borderTopLeftRadius: 16, borderTopRightRadius: 16, padding: 16 },
  feuilleTitre: { fontSize: 18, fontWeight: '600' },
  note: { fontSize: 14, marginBottom: 8 },
  choix: { paddingVertical: 15, borderTopWidth: StyleSheet.hairlineWidth },
});
