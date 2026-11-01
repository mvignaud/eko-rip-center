import { Tabs } from 'expo-router';
import { Text } from 'react-native';
import { AlertesProvider, useAlertes } from '../../src/alertes';
import { useTheme } from '../../src/theme';

// Icônes simples en glyphes, pour ne pas ajouter de bibliothèque
const icone = (glyphe: string) =>
  function Icone({ focused }: { focused: boolean }) {
    return <Text style={{ fontSize: 22, opacity: focused ? 1 : 0.55 }}>{glyphe}</Text>;
  };

function Onglets() {
  const t = useTheme();
  const { nonVus } = useAlertes();
  return (
    <Tabs
      screenOptions={{
        tabBarActiveTintColor: t.accent,
        tabBarInactiveTintColor: t.texteDiscret,
        tabBarStyle: { backgroundColor: t.carte, borderTopColor: t.separateur },
        headerStyle: { backgroundColor: t.carte },
        headerTitleStyle: { color: t.texte },
        headerShadowVisible: false,
        sceneStyle: { backgroundColor: t.fond },
      }}>
      <Tabs.Screen name="index" options={{ title: 'File', tabBarIcon: icone('📋') }} />
      <Tabs.Screen name="machines" options={{ title: 'Machines', tabBarIcon: icone('🖨️') }} />
      <Tabs.Screen
        name="alertes"
        options={{
          title: 'Alertes',
          tabBarIcon: icone('🔔'),
          tabBarBadge: nonVus > 0 ? (nonVus > 99 ? '99+' : nonVus) : undefined,
        }}
      />
      <Tabs.Screen name="reglages" options={{ title: 'Réglages', tabBarIcon: icone('⚙️') }} />
    </Tabs>
  );
}

export default function TabsLayout() {
  return (
    <AlertesProvider>
      <Onglets />
    </AlertesProvider>
  );
}
