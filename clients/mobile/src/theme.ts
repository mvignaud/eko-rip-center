import { useColorScheme } from 'react-native';

// Palette claire / sombre proche des couleurs système iOS
const clair = {
  fond: '#F2F2F7',
  carte: '#FFFFFF',
  texte: '#1C1C1E',
  texteDiscret: '#6C6C70',
  separateur: '#D1D1D6',
  accent: '#0A84FF',
  erreur: '#D70015',
  alerte: '#C93400',
  ok: '#248A3D',
  info: '#0A84FF',
  piste: '#E5E5EA',
};

const sombre: typeof clair = {
  fond: '#000000',
  carte: '#1C1C1E',
  texte: '#FFFFFF',
  texteDiscret: '#98989D',
  separateur: '#38383A',
  accent: '#0A84FF',
  erreur: '#FF453A',
  alerte: '#FF9F0A',
  ok: '#30D158',
  info: '#64D2FF',
  piste: '#3A3A3C',
};

export type Theme = typeof clair;

export function useTheme(): Theme {
  return useColorScheme() === 'dark' ? sombre : clair;
}
