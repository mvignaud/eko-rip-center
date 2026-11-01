// Mode démo : données fictives en mémoire, servies à la place du serveur RIP (relecture Apple / Google, captures).
// Les actions modifient ces données pour que l'interface réagisse comme avec un vrai serveur.
import type { EtatMobile, Evenement, InfoSession, Machine, Sortie } from './api';

let sorties: Sortie[] = [];
let evenements: Evenement[] = [];
let machines: Machine[] = [];
let prochainId = 100;

const maintenant = () => Date.now() / 1000;

function sortie(p: Partial<Sortie> & Pick<Sortie, 'id' | 'machine' | 'nom' | 'etat'>): Sortie {
  return {
    operation: 'impression', travail: `travail-${p.id}`, page: 1, code_erreur: null, erreur: null,
    cree_le: maintenant() - 3600, debut: null, fin: null, archivee: false, duree_min: null,
    raison_attente: null, planche_nom: null, fiche: {}, ...p,
  };
}

function evenement(p: Pick<Evenement, 'id' | 'genre' | 'gravite' | 'titre' | 'texte'> & Partial<Evenement>, ilYaS: number): Evenement {
  return { le: maintenant() - ilYaS, machine: null, sortie: null, travail: null, ...p };
}

/** Repart des données de départ (appelé à l'entrée en mode démo). */
export function reinitialiserDemo() {
  const t = maintenant();
  prochainId = 100;
  machines = [
    {
      id: 'epson', nom: 'Epson SC-S80600', modele: 'Imprimante éco-solvant 64 pouces',
      operations: ['impression'], snmp: true, joignable: true, releve_le: t,
      encres: [
        ['BK', 62], ['C', 48], ['M', 4], ['Y', 71], ['LC', 55], ['LM', 39], ['OR', 80], ['GR', 66], ['WH', 12],
      ].map(([encre, pourcent]) => ({ encre: encre as string, pourcent: pourcent as number })),
      longueur_restante_m: 37.5, alertes: ['Fin de rouleau proche'],
      en_cours: null, en_attente: 0, suspendues: 0,
    },
    {
      id: 'mutoh', nom: 'Mutoh Kona', modele: 'Table de découpe Kona',
      operations: ['decoupe'], snmp: false, joignable: null, releve_le: null,
      encres: [], longueur_restante_m: null, alertes: [], en_cours: null, en_attente: 0, suspendues: 0,
    },
    {
      id: 'graphtec', nom: 'Graphtec FC9000', modele: 'Traceur de découpe FC9000',
      operations: ['decoupe'], snmp: false, joignable: null, releve_le: null,
      encres: [], longueur_restante_m: null, alertes: [], en_cours: null, en_attente: 0, suspendues: 0,
    },
  ];
  sorties = [
    sortie({ id: 's1', machine: 'epson', nom: 'Bâche_Salon_3x2m.pdf', etat: 'en_cours', debut: t - 18 * 60, duree_min: 42, cree_le: t - 40 * 60 }),
    sortie({ id: 's2', machine: 'epson', nom: 'Vitrophanie_Boulangerie.pdf', etat: 'en_attente', duree_min: 25, cree_le: t - 30 * 60 }),
    sortie({ id: 's3', machine: 'epson', nom: 'Affiche_Concert_A0.pdf', etat: 'suspendue', duree_min: 12, cree_le: t - 25 * 60, raison_attente: 'Changer le support : papier photo mat' }),
    sortie({
      id: 's4', machine: 'epson', nom: 'Kakemono_Salon_Habitat.pdf', etat: 'echec', code_erreur: 'bourrage',
      erreur: 'Bourrage support détecté — vérifier le rouleau', cree_le: t - 70 * 60, debut: t - 40 * 60, fin: t - 26 * 60,
    }),
    sortie({ id: 's5', machine: 'epson', nom: 'Panneau_Chantier_120x80.pdf', etat: 'terminee', cree_le: t - 90 * 60, debut: t - 35 * 60, fin: t - 12 * 60 }),
    sortie({ id: 's6', machine: 'graphtec', nom: 'Stickers_Ronds_500ex.pdf', etat: 'en_attente', operation: 'decoupe', duree_min: 18, cree_le: t - 20 * 60 }),
    sortie({ id: 's7', machine: 'graphtec', nom: 'Étiquettes_Pots_de_Miel.pdf', etat: 'en_attente', operation: 'decoupe', duree_min: 9, cree_le: t - 15 * 60 }),
    sortie({ id: 's8', machine: 'mutoh', nom: 'Lettrage_Camion_Dupont.pdf', etat: 'terminee', operation: 'decoupe', cree_le: t - 3 * 3600, debut: t - 2.5 * 3600, fin: t - 2 * 3600 }),
  ];
  const epson = { machine: 'epson' };
  evenements = [
    evenement({ id: 8, genre: 'encre_basse', gravite: 'alerte', titre: 'Encre basse : magenta (M)', texte: 'Il reste 4 % d\'encre magenta sur Epson SC-S80600.', ...epson }, 3 * 60),
    evenement({ id: 7, genre: 'sortie_terminee', gravite: 'info', titre: 'Sortie terminée', texte: 'Panneau_Chantier_120x80.pdf (page 1) sur Epson SC-S80600.', sortie: 's5', ...epson }, 12 * 60),
    evenement({ id: 6, genre: 'sortie_echec', gravite: 'erreur', titre: 'Sortie en échec', texte: 'Kakemono_Salon_Habitat.pdf : bourrage support détecté.', sortie: 's4', ...epson }, 26 * 60),
    evenement({ id: 5, genre: 'alerte_machine', gravite: 'erreur', titre: 'Epson SC-S80600 : alerte', texte: 'Capot ouvert', ...epson }, 41 * 60),
    evenement({ id: 4, genre: 'machine_revenue', gravite: 'info', titre: 'Epson SC-S80600 de nouveau joignable', texte: 'La machine répond de nouveau sur le réseau.', ...epson }, 65 * 60),
    evenement({ id: 3, genre: 'machine_injoignable', gravite: 'erreur', titre: 'Epson SC-S80600 injoignable', texte: 'Plus de réponse de la machine depuis 5 minutes.', ...epson }, 72 * 60),
    evenement({ id: 2, genre: 'sortie_terminee', gravite: 'info', titre: 'Sortie terminée', texte: 'Lettrage_Camion_Dupont.pdf (page 1) sur Mutoh Kona.', sortie: 's8', machine: 'mutoh' }, 2 * 3600),
    evenement({ id: 1, genre: 'encre_basse', gravite: 'alerte', titre: 'Encre basse : blanc (WH)', texte: 'Il reste 12 % d\'encre blanche sur Epson SC-S80600.', ...epson }, 3 * 3600),
  ];
}

function etat(): EtatMobile {
  return {
    version: 'démo',
    dernier_evenement: evenements[0]?.id ?? 0,
    machines: machines.map((m) => {
      const actives = sorties.filter((s) => s.machine === m.id);
      const cours = actives.find((s) => s.etat === 'en_cours');
      return {
        ...m,
        releve_le: m.snmp ? maintenant() - 20 : null,
        en_cours: cours ? { sortie: cours.id, nom: cours.nom, page: cours.page, debut: cours.debut } : null,
        en_attente: actives.filter((s) => s.etat === 'en_attente').length,
        suspendues: actives.filter((s) => s.etat === 'suspendue').length,
      };
    }),
  };
}

function action(id: string, nom: string) {
  const i = sorties.findIndex((s) => s.id === id);
  const s = sorties[i];
  if (!s) throw new Error('Élément introuvable (déjà retiré ?).');
  switch (nom) {
    case 'reprendre':
      // une seule sortie en cours par machine : l'éventuelle précédente repasse en attente
      sorties.forEach((x) => { if (x.machine === s.machine && x.etat === 'en_cours') x.etat = 'en_attente'; });
      s.etat = 'en_cours';
      s.debut ??= maintenant();
      break;
    case 'suspendre': s.etat = 'suspendue'; break;
    case 'annuler': s.etat = 'annulee'; s.fin = maintenant(); break;
    case 'en_tete': sorties.splice(i, 1); sorties.unshift(s); break;
    case 'refaire':
      sorties.push({
        ...s, id: `s${++prochainId}`, etat: 'en_attente', erreur: null, code_erreur: null,
        cree_le: maintenant(), debut: null, fin: null,
      });
      break;
    default: throw new Error('Action impossible dans l\'état actuel de la sortie.');
  }
}

/** Remplace le serveur : même chemins et mêmes formes de réponse que l'API réelle. */
export async function reponseDemo<T>(methode: string, chemin: string): Promise<T> {
  const [route] = chemin.split('?');
  let r: unknown;
  if (methode === 'GET' && route === '/api/session') {
    r = { acces_libre: true, utilisateur: null, role: 'admin' } satisfies InfoSession;
  } else if (methode === 'GET' && route === '/api/file') {
    r = sorties.map((s) => ({ ...s }));
  } else if (methode === 'GET' && route === '/api/mobile/etat') {
    r = etat();
  } else if (methode === 'GET' && route === '/api/mobile/evenements') {
    r = { evenements: evenements.map((e) => ({ ...e })), dernier: evenements[0]?.id ?? 0 };
  } else {
    const m = /^\/api\/file\/([^/]+)\/([^/]+)$/.exec(route);
    if (methode === 'POST' && m) action(decodeURIComponent(m[1]), m[2]);
    // appareils / essai de notification : sans objet en démo
  }
  return r as T;
}
