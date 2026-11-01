// Couleurs usuelles des encres ; code inconnu = gris
const COULEURS: Record<string, string> = {
  BK: '#2B2B2B', K: '#2B2B2B', MK: '#2B2B2B', PK: '#2B2B2B',
  LK: '#8E8E93', LLK: '#B8B8BD', GY: '#8E8E93',
  C: '#00AEEF', LC: '#7FD6F5',
  M: '#EC008C', LM: '#F59BC8',
  Y: '#F5C400',
  OR: '#FF8A00', GR: '#3BB54A', R: '#E53935', B: '#3F51B5', VI: '#8E44AD',
  WH: '#E8E8EC', MS: '#AEB6BF', SI: '#AEB6BF',
};

export function couleurEncre(code: string): string {
  return COULEURS[code.toUpperCase()] ?? '#8E8E93';
}
