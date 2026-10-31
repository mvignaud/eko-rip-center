import React from 'react';
import { Pressable, StyleSheet, Text, View, ViewStyle } from 'react-native';
import { useTheme } from './theme';

export function Carte({ children, style }: { children: React.ReactNode; style?: ViewStyle }) {
  const t = useTheme();
  return <View style={[styles.carte, { backgroundColor: t.carte }, style]}>{children}</View>;
}

export function Badge({ texte, couleur }: { texte: string; couleur: string }) {
  return (
    <View style={[styles.badge, { backgroundColor: couleur + '26' }]}>
      <Text style={[styles.badgeTexte, { color: couleur }]}>{texte}</Text>
    </View>
  );
}

export function Bouton({ titre, onPress, couleur, desactive }: {
  titre: string; onPress: () => void; couleur?: string; desactive?: boolean;
}) {
  const t = useTheme();
  return (
    <Pressable
      onPress={onPress}
      disabled={desactive}
      style={({ pressed }) => [
        styles.bouton, { backgroundColor: couleur ?? t.accent, opacity: desactive ? 0.4 : pressed ? 0.7 : 1 },
      ]}>
      <Text style={styles.boutonTexte}>{titre}</Text>
    </Pressable>
  );
}

/** Bandeau d'erreur avec bouton Réessayer. */
export function BlocErreur({ message, onRetry }: { message: string; onRetry?: () => void }) {
  const t = useTheme();
  return (
    <View style={[styles.erreur, { backgroundColor: t.erreur + '1F' }]}>
      <Text style={{ color: t.erreur, fontSize: 15 }}>{message}</Text>
      {onRetry && (
        <Pressable onPress={onRetry} hitSlop={8}>
          <Text style={{ color: t.accent, fontWeight: '600', marginTop: 6, fontSize: 15 }}>Réessayer</Text>
        </Pressable>
      )}
    </View>
  );
}

export function Vide({ texte }: { texte: string }) {
  const t = useTheme();
  return <Text style={[styles.vide, { color: t.texteDiscret }]}>{texte}</Text>;
}

const styles = StyleSheet.create({
  carte: { borderRadius: 14, padding: 14, marginHorizontal: 16, marginBottom: 12 },
  badge: { borderRadius: 6, paddingHorizontal: 8, paddingVertical: 2, alignSelf: 'flex-start' },
  badgeTexte: { fontSize: 12, fontWeight: '600' },
  bouton: { borderRadius: 12, paddingVertical: 14, alignItems: 'center' },
  boutonTexte: { color: '#fff', fontSize: 17, fontWeight: '600' },
  erreur: { borderRadius: 12, padding: 12, marginHorizontal: 16, marginVertical: 8 },
  vide: { textAlign: 'center', marginTop: 40, fontSize: 15 },
});
