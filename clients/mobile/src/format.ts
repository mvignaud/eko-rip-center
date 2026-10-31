// Formatage de dates et durées en français

export function ilYa(ts: number, maintenant = Date.now() / 1000): string {
  const s = Math.max(0, Math.round(maintenant - ts));
  if (s < 45) return "à l'instant";
  const min = Math.round(s / 60);
  if (min < 60) return `il y a ${min} min`;
  const h = Math.round(min / 60);
  if (h < 24) return `il y a ${h} h`;
  const j = Math.round(h / 24);
  if (j < 7) return `il y a ${j} j`;
  return new Date(ts * 1000).toLocaleDateString('fr-FR');
}

export function duree(secondes: number): string {
  const min = Math.max(0, Math.round(secondes / 60));
  if (min < 1) return 'moins d’1 min';
  if (min < 60) return `${min} min`;
  const h = Math.floor(min / 60);
  const m = min % 60;
  return m ? `${h} h ${String(m).padStart(2, '0')}` : `${h} h`;
}

export function metres(m: number): string {
  return `${m.toLocaleString('fr-FR', { maximumFractionDigits: 1 })} m`;
}
