# E-KO RIP Center — roadmap de parité ONYX Thrive (⊃ Go)

Date : 2026-10-24. État du code : v0.13.0 (CHANGELOG) sur `eko-rip/`.

## Sources

| Fichier | Contenu | Poids |
|---|---|---|
| `references/onyx-go-25/support-center-rip.md` | centre de support ONYX (156 articles RIP) | **fait foi** en cas de conflit |
| `references/onyx-go-25/job-editor.md` | Job Editor (aide publique v25) | |
| `references/onyx-go-25/rip-queue.md` | file, Quick Sets, ports, archive | corrigé le 24/10 (pool) |
| `references/onyx-go-25/medias-couleur.md` | Media Manager, calibration, ICC, tons directs | |
| `references/onyx-go-25/layout-imbrication.md` | placement, Layout Tool, step & repeat, mosaïque | |
| `references/onyx-go-25/cut-server.md` | CUT-Server, print & cut | corrigé le 24/10 (miroir/échelle/copies) |
| `references/onyx-go-25/sommaire-et-reste.md` | sommaire, coûts, transverse | |
| `references/onyx-go-25/support-center-align.md` | ONYX Align (MIS) | seul le § 4 (pont côté RIP) est repris |
| `references/caldera/synthese-caldera.md` | synthèse documentaire CalderaRIP V19.3 (sources publiques) ; § 3 = items C0 à C18 | ajoutés le 24/10, rangés par phase (mention « Caldera Cx ») |
| `todo-rip-multiplateforme.md`, `eko-rip/CHANGELOG.md`, `eko-rip/serveur/src/*.rs`, `static/index.html` | état réel | vérifié par grep le 24/10 |

## Règle de non-copie (rappel, s'applique à chaque item)

- On reproduit des **fonctions** et une **logique d'usage**. L'UX s'**inspire** d'ONYX (disposition : file à gauche,
  aperçu au centre, inspecteur à droite) sans en reprendre textes, libellés, logos, icônes, captures.
- Jamais : décompiler un binaire ONYX, reprendre un profil, un pilote, une table couleur, un `.set`, un `.OML`, une
  mire ONYX, un nuancier ONYX. La base `db/` d'ONYX sert au plus de **donnée observée**, jamais de code.
- Protocoles machine : documentation publique du fabricant, ou **observation de nos propres machines** (captures
  USB/réseau déjà faites pour Kona et Epson).
- Valeurs par défaut chiffrées (ex. N-factor 2) : on les choisit nous-mêmes, la doc ONYX sert de repère.

## Légende

- État : **Fait** (fonction présente et éprouvée par test ou à l'atelier) · **Partiel** (présente mais incomplète
  ou non éprouvée) · **Absent**.
- Priorité : **P1** indispensable à la production de l'atelier · **P2** valeur métier forte · **P3** confort ou
  différé.
- Preuve : fichier `serveur/src/…` ou route API ; `index.html` = `serveur/static/index.html`.
- 🏭 = demande la machine ou l'atelier.

---

# 1. Matrice de couverture

## 1.1 File RIP / Queue

| Fonction ONYX | État | Preuve / manque |
|---|---|---|
| Une file par machine, ordre, en tête, monter, descendre | Fait | `file.rs` (`Etat`, `prochaine`), `POST /api/file/{id}/{action}` |
| Attente opérateur (Hold), reprise | Fait | `file.rs` `Etat::Suspendue` ; `reglages.rs` `Auto.suspendre` |
| Réimpression sans re-RIP depuis l'historique | Fait | action « Refaire » (CHANGELOG 0.7.0) |
| Sortie coupée par un arrêt → échec (pas de reprise au milieu) | Fait | `file.rs` `File::lire` (`interrompu`) |
| RIP anticipé : le travail est rendu avant d'entrer en file | Fait | rendu à l'import (`main.rs` `Statut`), la file ne fait que sortir |
| Quick Sets = réglages enregistrés, appliqués à l'import | Fait | `reglages.rs`, `/api/reglages` |
| Dossier chaud par réglage | Partiel | `main.rs:1411` ; manquent les règles préfixe/taille (R), la copie au lieu du déplacement |
| Import/export de réglages choisis (multi-réglages, doublon écraser/renommer) | Partiel | `sauvegarde.rs` = toute la config, sans sélection |
| Import multiple, PDF multipage | Fait | `POST /api/travaux`, pages par travail |
| Une page par travail (case) | Absent | |
| Formats d'entrée TIFF, JPEG, PNG, PSD, EPS | Absent | `main.rs:51` `Format` = `Pdf`, `Svg` (AI via PDF) |
| **Attente de support** (support/laize du travail ≠ machine) | Absent | aucun champ « support chargé » sur `Machine` |
| Pool d'imprimantes (même support obligatoire, sinon machine dupliquée) | Absent | |
| Rouleau actif (machine multi-rouleaux) | Absent | pas de machine concernée au parc |
| Tampon après impression : archiver selon la place / indéfiniment / supprimer | Partiel | purge par âge `main.rs:1361` `a_purger` ; pas de seuil d'espace |
| Impression pendant le RIP, % de cache avant envoi | Absent | dépend du pilote Epson |
| Compteurs, filtres « toutes machines / une machine », recherche | Partiel | recherche + filtres (`index.html`, 0.7.0) ; pas de poids du tampon |
| Fiche travail (commande, client, échéance, quantité due, notes) | Absent | grep `commande`/`client` vide |
| Journal des impressions (date, support, opérateur, encre, surface) | Partiel | `Sortie.debut/fin` (`file.rs`) ; ni opérateur ni rétention dédiée |
| Copier vers une autre machine | Partiel | `POST /api/file` vers toute machine du catalogue ; pas de copie de réglages |
| Épreuve sur une 2e machine (« -Proof ») | Absent | BAT PDF fait : `bat.rs`, `/api/travaux/{id}/bat.pdf` |
| Durée estimée, notifications | Fait | `machines.rs` `m2_heure` ; `index.html:1053` |
| Diagnostic « pourquoi ce travail ne part pas » | Partiel | `Sortie.code_erreur` ; pas de raison d'attente explicite |
| Statut API « pas prêt » ≠ « inexistant » | Partiel | 404 `introuvable` ; pas de « pas prêt » |
| Portail web, imprimante virtuelle, JDF | Absent | volontaire : interface dans les clients seulement (décision D du 24/10) |

## 1.2 Job Editor / éditeur visuel

| Fonction ONYX | État | Preuve / manque |
|---|---|---|
| Travail séparé de la source, édition non destructive | Fait | `/api/travaux/{id}/editer`, `/reglages` : rendu refait depuis l'original |
| Grand aperçu | Partiel | fiche à grand aperçu (0.12.1) ; visionneuse `<img>` `index.html:1170` ; ni zoom, ni loupe, ni règles, ni position curseur |
| Échelle, rotation 0/90/180/270, miroir, marge | Fait (numérique) | `edition.rs` `Edition {echelle, rotation, miroir, marge_mm}` ; aucun geste sur l'aperçu |
| Largeur/hauteur liées ou déliées | Absent | échelle unique |
| Recadrage (cadre à poignées, L/H/X/Y, ancrage, verrou) | Absent | |
| Rotation auto dans la laize | Fait | `reglages.rs` `rotation_auto`, `main.rs:557` |
| Mosaïque : lés égaux, recouvrement, n° et traits d'assemblage | Fait | `mosaique.rs` `les`, `composer(avec_reperes)` |
| Mosaïque : colonnes × rangées, lés inégaux, frontières à la souris | Absent | lés verticaux seulement |
| Recouvrement et lés visibles à l'aperçu ; zone de soudure sans encre | Absent | |
| Choisir les lés à imprimer | Partiel | un lé = une page, mise en file page par page ; pas de cases |
| Fond perdu (couleur, premier pixel, miroir, duplication), trim box | Absent | |
| Œillets | Partiel | `edition.rs` `Oeillets {espacement_mm, retrait_mm, diametre_mm, bords}` ; ni quantité, ni 4 styles |
| Repères de coupe, repérage, pli, couture, angles | Partiel | `reperage.rs` = repères Kona seulement |
| Étiquette imprimée en marge (nom, date, L×H, lé, mode) | Absent | |
| Contour de découpe rectangulaire | Fait | `edition.rs` `contour_mm` |
| Filtres couleur (courbes, niveaux), remplacement de couleur, aperçu scindé | Absent | |
| Step & repeat (demi-saut) | Fait | `imbrication.rs` `grille`, `Motif` |
| Annuler / rétablir | Absent | (absent d'ONYX aussi) |

## 1.3 Médias & couleur

| Fonction ONYX | État | Preuve / manque |
|---|---|---|
| Profils ICC déposés, entrée CMJN/RVB, sortie, intention, ignorer les profils du PDF | Fait | `couleur.rs`, `/api/profils` |
| Épreuve écran du support | Fait | aperçu composite + BAT (`bat.rs`) |
| Profils image/vecteur séparés, BPC, teintes pures, intention propre aux tons directs | Absent | |
| Catalogue de supports (prix au m², rouleau) | Fait | `couverture.rs` `prix_m2` |
| **Chaîne Support → Mode** (encres, résolution, passes, trame, restriction, linéarisation, limites, ICC) | Absent | le support n'est qu'un prix |
| Séparation 9 encres (C Lc M Lm Y K Lk Or Rd) | Absent | grep `tramage`/`dither`/`ESC/P` vide |
| Restriction, linéarisation, limites d'encre, recalibration | Absent | 🏭 spectro |
| Profilage ICC mesuré (moteur libre) | Absent | 🏭 spectro |
| Blanc auto (sous-couche, aplat, rétraction, union) | Fait | `blanc.rs` `generer_tiff` |
| Blanc / argent sous ou sur la couleur | Fait | `epson.rs` `Couches`, `preparer` |
| Tons de découpe jamais imprimés (colorant `None`) | Fait | `decoupe.rs` `neutraliser` |
| Table de tons directs (Lab, alias, casse et espaces exacts), remplacement par mode | Absent | |
| Nuanciers, contrôle de constance (ColorCheck), G7 | Absent | |
| Lecture des EMX Epson | Absent | |

## 1.4 Imposition / Layout / imbrication

| Fonction ONYX | État | Preuve / manque |
|---|---|---|
| Planche multi-travaux, copies, rangées, espacement | Fait | `imbrication.rs` `placer`, `POST /api/planches` |
| Imbrication sur la forme, 0/90/180/270 | Fait | `forme.rs` `placer` |
| Aperçu de l'imposition avant création | Fait | `POST /api/planches/apercu` |
| **Imbrication dans la file** (stratégies un par un / groupé / économie) | Absent | planche à la main uniquement |
| Déclencheur manuel / temporisé / à la surface | Absent | |
| Regroupement par support et par découpeuse | Absent | |
| Contour de toute la planche (`NestTrimBox`) | Absent | |
| Repères de massicot X/Y, bande de purge, justification, décalage d'origine | Absent | |
| Planche manuelle (glisser-déposer, verrou, grille) | Absent | |
| Templates de cellules, données variables (CSV) | Absent | |
| Garde-fou : rotation auto seulement en résolution carrée | Absent | à prévoir avec le pilote Epson (modes non carrés) |

## 1.5 Découpe / Cut Server

| Fonction ONYX | État | Preuve / manque |
|---|---|---|
| Extraction des tons de découpe (PDF, formulaires, SVG par calque) | Fait | `decoupe.rs` `extraire`, `extraire_svg` |
| Reconnaissance par **préfixe** en début de nom + suffixe libre (`CutContour1…9`) | Partiel | `decoupe.rs:34` `est_ton_de_decoupe` = égalité sur une liste fixe |
| **Couches de découpe liées aux réglages par le nom** | Partiel | `machines.rs` `parametres_decoupe` ; un seul réglage par travail (`main.rs:1933`) ; outil par ton (`decoupe.rs` `outil`) |
| Ordre des couches = ordre de coupe ; activer/désactiver une couche | Partiel | `decoupe.rs` `ordonner` (outils puis intérieurs, plus proche) ; pas de couche manipulable |
| HP-GL au pas machine ; série, TCP, fichier | Fait | `machines.rs` `Connexion`, `plotter.rs` |
| Kona : AL5, cadres Types 1/2/3 + POSTNET, `Nom_SF\|MF_n.plt` | Fait (Type 2 éprouvé) / Partiel (Type 3) | `reperage.rs` ; Type 3 🏭 |
| Réglages de coupe (vitesse, force, accélération, outil, motif, diffusion) ; décalage X/Y ; découpe retournée | Fait | `machines.rs` `ParametreDecoupe`, `decalage_mm`, `decoupe_retournee` |
| Rotation quart de tour du travail de coupe, flèche de sens d'avance | Partiel | retournée (180°) seulement |
| Découpe seule : miroir, échelle, copies X/Y avec écarts, départ origine | Partiel | PDF : `edition.rs` (échelle, miroir) ; SVG : aucune édition ; copies X/Y absentes |
| Découpe auto à l'arrivée ; politique après coupe | Partiel | `Auto.decouper` ; archive = historique de file |
| Contour auto sur bord image / fond perdu / trim, choke | Partiel | rectangle `edition.rs` `contour_mm` |
| « Décaler la coupe » et « étendre l'image près des tracés » (25.5) | Absent | |
| Détourage automatique (Make Path) | Absent | |
| Alertes : ton de découpe en RVB, tracé avec fond, touchant la boîte, contour vectorisé (coupé 2 fois) | Absent | `controle.rs` ne contrôle que polices et ppi |
| Miroir des tracés en print & cut | Absent | ⚠️ support contradictoire : **à éprouver à l'atelier** |
| Fond sous repères, compensation de cintrage, rouleau à rouleau | Absent | |
| Graphtec FC7000, Roland VP (découpe) | Partiel | catalogue `machines.rs` ; 🏭 non éprouvés |

## 1.6 Pilotes imprimantes

| Fonction ONYX | État | Preuve / manque |
|---|---|---|
| Epson : état SNMP (laize, n° de support, encres, chauffages, alertes) | Fait | `media.rs` `lire_medias`, `lire_etat_epson` |
| Epson : plaques → canaux, découpe jamais envoyée | Partiel | `epson.rs` `preparer` : 5 canaux contone (hypothèse contredite par la capture) ; sortie en dossier |
| **Epson : séparation 9 encres + tramage 2 bits + émetteur ESC/P2 en TCP 9100** | Absent | capture analysée (todo § « CAPTURE ONYX → EPSON ») |
| Ports TCP 9100, série, périphérique, dossier | Fait | `machines.rs` `Connexion` |
| Port IPP (25.5), renvoi vers un autre RIP | Absent | |
| Roland VP : impression | Absent | 🏭 capture à faire |
| Cadence par machine | Fait | `machines.rs` `m2_heure` |

## 1.7 Administration / réseau / pannes

| Fonction ONYX | État | Preuve / manque |
|---|---|---|
| Comptes (Argon2id), rôles admin / opérateur, blocage | Fait | `acces.rs` |
| Sauvegarde / restauration de configuration | Fait | `sauvegarde.rs`, `/api/sauvegarde` |
| Écriture atomique des fichiers d'état (tmp + rename) | Fait | `file.rs:95`, `machines.rs:242`, `reglages.rs:161`, `couverture.rs:171`, `acces.rs:59` |
| **Fichier d'état illisible → reprise sur N-1, alerte** | Absent | ⚠️ `File::lire` : JSON illisible = `unwrap_or_default` = **file vide sans alerte** ; même schéma ailleurs |
| Dossiers de travail absents : recréés ou relocalisés | Partiel | `create_dir_all` ponctuels |
| Purge par âge, espace disque affiché | Fait | `a_purger`, `/api/systeme` |
| Seuil d'espace libre, estimation de place avant RIP | Absent | |
| Sauvegarde automatique avant mise à jour, rapport de migration | Absent | |
| ISO Ubuntu 3 profils, CI d'installation | Fait | CHANGELOG 0.10.0 / 0.11.0 |
| Accès réservé aux clients | Fait | `main.rs:1264` `client_requis` |
| Doc admin + FAQ ; glossaire | Fait | `docs/`, `glossaire.md` (9.7) |

## 1.8 Rapports / coûts

| Fonction ONYX | État | Preuve / manque |
|---|---|---|
| Couverture et coût **estimés** par page (encre + support, HT/TTC) | Fait | `couverture.rs` `estimer` |
| CSV de consommation estimée | Fait | `/api/rapports/consommation.csv` (`main.rs:329`) |
| Prix cartouche → €/l ; rouleau → €/m² | Fait | `couverture.rs` `prix_litre`, `prix_m2` |
| Relevé SNMP réel (baisses seules), sur une période | Fait | `releves.rs` `consommation(depuis, jusqua)`, `/api/machines/{id}/consommation` |
| **Consommation réelle par travail** (fenêtre début/fin) | Partiel | `Sortie.debut/fin` + `releves::consommation` existent, **non reliés** (attend le pilote) |
| Facteur de correction par résolution × encre | Absent | |
| CSV réel par période / par travail | Absent | |
| Étiquettes Zebra 100 × 150 mm | Absent | usage à préciser |

## 1.9 Pont ERP ↔ RIP (côté RIP)

| Échange | État | Preuve / manque |
|---|---|---|
| Catalogue machines lisible | Partiel | `GET /api/machines` ; réservé au client (`client_requis`), pas de modes ni d'événement |
| Réception d'un travail ERP (réf., fichier par URL signée + SHA-256, machine/support/mode) | Absent | `POST /api/travaux` = multipart du client |
| Statut par événement (webhook signé, séquence rejouable) | Absent | |
| Consommation réelle remontée | Absent | dépend 1.8 |
| Aperçu ripé | Partiel | `/api/travaux/{id}/pages/{n}/apercu.png` |
| Estimation synchrone (surface, longueur, durée, encre) | Partiel | `/api/planches/apercu` + durée estimée ; pas d'encre par canal en ml |

---

# 2. Roadmap par phases

Deux pistes en parallèle : **sans matériel** (phases 0, 1, 3, 4, 5, 7) et **atelier** 🏭 (phase 2, essais des phases 4 et 6).
Une phase ne démarre que si ses dépendances sont **finies** (critère vérifié), sauf mention « en parallèle ».

```
P0 fiabilité ─┬─> P1 supports/modes ─┬─> P2 pilote Epson 🏭 ──> P6 conso réelle + rapports ──> P7 pont ERP
              │                      ├─> P5 imbrication dans la file
              ├─> P3 éditeur visuel ─┘        ▲
              └─> P4 découpe (couches) ───────┘
P8 couleur mesurée 🏭 (après P2) · P9 reste (au fil de l'eau)
```

## Phase 0 — Socle de fiabilité (sans matériel)

Dépend de : rien. Débloque : tout.

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| 0.1 | P1 | ✅ FAIT v0.13.2 (`etat.rs`) — **Résistance aux coupures** des fichiers d'état (`file.json`, `machines.json`, `reglages.json`, `tarifs.json`, `utilisateurs.json`, `job.json`) : copie N-1 avant chaque écriture, `fsync` avant `rename` ; à la lecture, fichier illisible → reprise sur N-1 + alerte écran, jamais une valeur vide silencieuse | test : JSON tronqué au milieu → le serveur démarre, la file N-1 est rechargée, une alerte `etat_repris` est visible ; test : N et N-1 illisibles → démarrage refusé avec message (pas de file vide) |
| 0.2 | P1 | ✅ FAIT (main 9f1a26e) — **Dossiers absents** (données, entrées, sorties) : recréés au démarrage ; dossier surveillé disparu → alerte, pas de blocage | test : supprimer `entrees/<réglage>/` puis redémarrer → recréé, alerte |
| 0.3 | P1 | ✅ FAIT v0.13.3 (5 tests `regression_*` ; cas 5 « réduire pour tenir + fond perdu » sans équivalent → à ouvrir en phase 3 ; bogue CropBox/Rotate corrigé) — **Tests de non-régression géométrie** tirés des bogues ONYX corrigés : rotation auto + résolution non carrée + fond perdu + étiquette ; un fichier de coupe par feuille d'un travail multi-feuilles ; PDF multipage + boîte de rognage + une page par travail → une coupe par page ; échelle d'un travail tourné sur la bonne dimension ; « réduire pour tenir » + fond perdu sans rognage ; rotation auto refusée en résolution non carrée | 6 tests `cargo test` nommés `regression_*`, verts en CI ; chaque test échoue si on retire la correction |
| 0.4 | P1 | ✅ FAIT (db59e4e ; lien FAQ reporté : doc non servie) — **Raison d'attente explicite** par sortie : `attente_support`, `attente_operateur`, `format_trop_petit` (image + repères + recouvrement), `machine_hors_ligne`, `source_absente` ; affichée dans la file avec lien FAQ | test API : chaque cas renvoie son code ; écran : badge + lien |
| 0.5 | P2 | ✅ FAIT (c513bb5) — Statut API « pas prêt » (409 `pas_pret`) distinct de « inexistant » (404) | test sur une page en cours de rendu |
| 0.6 | P2 | ✅ FAIT (d7b0171) — **Seuil d'espace libre** (Mo) qui purge les plus vieux travaux imprimés, et refus anticipé d'un travail si la place estimée manque (≈ 1 Go/m² à 600 dpi, au prorata) | test : seuil franchi → purge des plus vieux inactifs seulement ; envoi refusé `espace_insuffisant` |
| 0.7 | P2 | ✅ FAIT (11e599d, éprouvé sur le serveur de test) — Sauvegarde automatique de la configuration avant mise à jour du paquet (`postinst`), liste de ce qui est repris | mise à jour en VM CI : fichier `sauvegardes/avant-<version>.json` présent |
| 0.8 | P3 | (Caldera C14) Compléter 0.6 : **deux seuils** d'espace libre (avertir, refuser), envoi automatique refusé = travail en erreur avec motif ; purge propre aux travaux venus de l'API (ne pas conserver après impression, option) | test : seuil bas → avertissement, seuil critique → refus ; travail API purgé à l'impression si option active |

## Phase 1 — Chaîne support → modes d'impression (sans matériel, données)

Dépend de : 0.1. Débloque : 2, 5, 6, 7, 8.

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| 1.1 | P1 | ✅ FAIT v0.14.0 (`supports.rs` ; prix laissés dans tarifs.json, liés par nom jusqu'à 1.2 ; écran d'édition à faire avec 1.3) — **Modèle Support → Mode** par machine : Support = groupe (configuration d'encres : `OR LK BK C M Y R LC LM`, ± WH/MS) + nom + laize + prix ; Mode = résolution, passes, trame, réglages machine (chauffage, vitesse), plus tard restriction/linéarisation/limites/ICC. Identifiant stable (UUID) + nom affiché | `supports.json` versionné, API `GET/POST /api/machines/{id}/supports` ; migration des supports actuels de `tarifs.json` sans perte (test) |
| 1.2 | P1 | ✅ FAIT (6343116) — Le travail référence **support + mode** (par id) ; le rendu prend la résolution du mode ; règle « modifier une étape invalide les suivantes » (indicateur « re-RIP requis ») | test : changer le mode d'un travail rendu → indicateur re-RIP + rendu refait |
| 1.3 | P1 | ✅ FAIT (fefec69 ; rendu visuel à vérifier) — **Support chargé** par machine : saisi, ou lu par SNMP (n° de support Epson ST2 0x2d, laize Printer-MIB) | Système > Machines affiche le support chargé ; SNMP Epson le met à jour (test avec relevé enregistré) |
| 1.4 | P1 | ✅ FAIT sauf options globales « ne pas ripper » (rendu à l'envoi aujourd'hui) — **Attente de support** : sortie bloquée `attente_support` tant que support/laize du travail ≠ machine ; ne se lève pas à la main ; options globales « ne pas ripper les travaux en attente / en attente de support » (défaut : on rippe) | test file : support différent → bloqué ; changement de support d'un côté → part |
| 1.5 | P1 | ✅ FAIT (ed1f869) — (données HEXIS fournies par Mathieu le 24/10 : base de profils ICC en ligne https://www.hexis-graphics.com/fr/supports/impression-numerique/profils-icc/ — 2 856 profils par marque/modèle/encre/RIP/support, à parcourir au navigateur ; tableaux « profils croisés » éco-solvant et latex 2024-02 dans `tasks/references/hexis/` = profil de remplacement 1er/2e choix d'un support sans profil propre, avec remarques encre/séchage/chromie → catalogue de supports HEXIS importable ; ICC jamais dans le dépôt ni les paquets ; jamais un fichier média au format ONYX ; piste de Mathieu (24/10) : profils HEXIS « Caldera » — utilisables seulement s'ils sont livrés en .icc standard, comme profil PROVISOIRE (épreuvage, point de départ), car un profil caractérise la séparation/linéarisation du RIP qui l'a fait ; jamais extraits d'un paquet propre à un RIP ; profils de production = les nôtres, i1Pro, phase 8) Export / import d'un **paquet support** (support + modes + réglages enregistrés liés), un seul fichier maison, avec version | aller-retour export → import sur un serveur vierge = identique (test) ; import refusé si version plus récente que le serveur |
| 1.6 | P2 | ✅ FAIT (6d0631d) — Import/export de **réglages choisis**, doublon : écraser ou renommer | test des deux choix |
| 1.7 | P2 | ✅ FAIT (8412e9d) — **Pool d'imprimantes** : une file logique sur N machines identiques, envoi à la première prête ; **même support obligatoire** (sinon `support_different`, cf. support-center-rip.md) ; pour des supports différents, « dupliquer la machine » (file et dossiers séparés) | test : 2 machines simulées en dossier → répartition ; support différent → refus d'entrer dans le pool |
| 1.8 | P3 | Rouleau actif pour machine multi-rouleaux | quand une telle machine arrive |

> Note (24/10, synthèse Caldera § 4) : les profils HEXIS « Caldera » sont livrés en **`.calpatch`** (ou `.oms` HP),
> paquet propre au RIP, pas en `.icc` standard → **jamais utilisés ni extraits**. La condition posée en 1.5 (« seulement
> s'ils sont livrés en .icc standard ») n'est donc pas remplie. Seuls les faits publiés du guide HEXIS (chauffage,
> passes, ΔE) peuvent nourrir nos supports ; profils de production = les nôtres (phase 8).

## Phase 2 — Pilote Epson SC-S80600 🏭 (capture déjà faite)

Dépend de : 1.1, 1.2 (le mode porte résolution et passes). En parallèle de 3, 4.

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| 2.1 | P1 | ✅ FAIT v0.15.0 (`escp2.rs` ; incertains : canaux 7/10, ordre des points, placement x → 2.4) — **Émetteur ESC/P2** (sans matériel d'abord) : en-tête EJL/remote mode, `ESC (G`, `(i` microweave, `(e` point variable, `(U`, `(C/(c/(S`, `(D`, bandes de 16 lignes `(v`/`($`/`ESC i` PackBits 2 bits, fin FF/LD/JE ; canaux 0 K, 1 M, 2 C, 4 Y, 16/17/18 Lk/Lm/Lc, 7 et 10 Or/Rd | test : une image synthétique émise → flux dont la **structure** (séquence de commandes, tailles de bande, numéros de canaux) égale celle de la capture `Stickers_Lions_club` ; aucun octet de la capture copié dans le dépôt |
| 2.2 | P1 | ✅ FAIT (separation.rs ; courbes à afficher dans Système avec 2.6 ; étalonnage papier 2.8) — **Séparation 9 encres** : C→C+Lc, M→M+Lm, K→K+Lk, Or et Rd depuis le gamut étendu ; par encre Start / Max / End / Transition (valeurs à nous) ; densité **monotone croissante** sur tout le canal | test : rampe 0→100 % par canal logique → somme pondérée monotone ; courbes visibles dans Système |
| 2.3 | P1 | ✅ FAIT (tramage.rs, aplats ±0,3 %, pic 3,8 < 6) — **Tramage 2 bits** (4 tailles de point) : diffusion d'erreur multi-niveaux par canal, courbe de répartition S→M→L | test : aplat 50 % → couverture mesurée 50 ± 2 % ; pas de motif périodique détecté (FFT) sur un dégradé |
| 2.4 | P1 | 🟡 1er TIRAGE RÉUSSI le 25/10 00:05 (aplat CMJN 4 × 50 mm, couleurs justes confirmées par Mathieu, x = 0 au bord du rouleau ; 194 mm consommés) — reste : fines bandes dans le jaune et bande plus foncée en tête (microweave / 1re avance), alertes SNMP pendant l'envoi à éprouver (capot) — Envoi TCP 9100 depuis la file, état par SNMP pendant l'impression, échec si l'imprimante répond une alerte | 🏭 **un aplat CMYK 100 × 100 mm imprimé** sur HEXIS, sans bande ; photo + relevé SNMP |
| 2.5 | P1 | WH / MS : capture d'un travail blanc puis argent, ajout des canaux | 🏭 capture faite, numéros de canaux WH/MS confirmés, essai blanc sous couleur sur transparent |
| 2.6 | P1 | ✅ FAIT (branche feat/pilote-epson) — Remplacer l'ancienne sortie contone (`epson.rs` 5 canaux) ; la sortie dossier reste un mode « Dossier » | `epson.rs` ne décrit plus le contone ; test de bout en bout PDF → flux ESC/P2 |
| 2.7 | P2 | Modes 360×720, 720×720, 720×1440, 1440×1440 ; garde-fou rotation auto en résolution non carrée (cf. 0.3) | 🏭 un essai par mode ; test du garde-fou |
| 2.8 | P2 | ⏸ BLOQUÉ (25/10) : Ghostscript tiffsep n'écrit la plaque qu'en fin de page → il faut d'abord un rendu par bandes (nouvel item à créer) ; le pilote, lui, envoie déjà au fil des bandes une fois le rendu fini — Impression pendant le RIP et « % de cache avant envoi » (0–100 %), désactivés dès qu'une découpe est affectée | test : 0 % → envoi au fil des bandes ; découpe affectée → 100 % forcé |
| 2.9 | P3 | Lecture des EMX (résolution, passes, chauffage) pour pré-remplir les modes | un EMX fourni par Mathieu lu sans erreur |
| 2.10 | P2 | ✅ FAIT le 31/10 côté serveur (réglage `empilement`, 5 couches au plus parmi couleur / blanc / argent ; `epson::preparer` rend les passages dans l'ordre déclaré, refus si couche sans plaque ou plaque sans couche ; repères sur fond blanc déjà couverts par 4.15) — reste à l'imprimante : l'ordre des passages dans le flux ESC/P2, après la capture WH/MS (2.5) — (Caldera C9) **Multicouche générique** au-delà de blanc sous/sur : N couches ordonnées (ex. blanc-couleur-blanc pour vitrophanie, primaire), et **repères blancs / fond sous repères sur transparent ou support foncé** (complète 2.5 et 4.15) | test : travail 3 couches → 3 plaques dans l'ordre déclaré ; repères sur fond blanc présents sur transparent |

## Phase 2 bis — Écran principal façon RIP-Queue (demandé le 25/10, prioritaire)

| # | Prio | Point | Fini quand |
|---|---|---|---|
| 2b.1 | P1 | ✅ 25/10 — Vue « Production » unique : machines en haut (état, support, encres), « À imprimer », « Imprimés » repliable avec compteurs, boutons d'action verticaux | captures 1440/1100 px FR/EN clair/sombre ; parcours import → file → impression sans changer de vue |
| 2b.2 | P1 | ✅ 25/10 — Inspecteur à onglets : Travail, Machine, Réglages (dépôt sur un réglage), Imbrication, Journal | captures ; dépôt sur un réglage = travail créé avec ce réglage |
| 2b.3 | P2 | ✅ 25/10 — Clic droit par zone, colonnes au choix, réordonner la file au glisser-déposer | captures |

## Phase 3 — Éditeur visuel façon Job Editor (sans matériel)

Dépend de : 0.1 ; 3.C dépend de la mosaïque existante ; 3.D de 4.1 pour les repères de coupe. En parallèle de 2.
Principe : chaque geste sur l'aperçu écrit les **mêmes champs** que l'édition numérique actuelle (`edition.rs`) ; le
rendu de production reste fait par le serveur. Disposition inspirée : outils à gauche, aperçu au centre, propriétés à
droite, barre d'état en bas (sans reprendre libellés ni icônes ONYX).

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| 3.A1 | P1 | ✅ FAIT v0.15.0 — **Aperçu grand format** : zoom (boutons, %, ajuster, molette), loupe (clic / Maj+clic / rectangle), déplacement, règles en mm, position du curseur, laize de la machine dessinée | test UI (client) : zoom 25→800 % sans perte de repère ; règle = dimensions réelles ± 0,5 mm sur une page de 1000 mm |
| 3.A2 | P1 | ✅ FAIT (3346e1b) — **Sélection** : clic sur la page → poignées ; pages d'un multipage en vignettes | sélection visible ; vignettes = nombre de pages |
| 3.A3 | P1 | ✅ FAIT (3346e1b ; repères dans la plaque Black : pas de couche séparée) — Aperçu des couches : image, découpe (magenta), blanc, argent, repères, activables | chaque couche se masque seule |
| 3.B1 | P1 | ✅ FAIT (7721fdb ; cadre commun à toutes les pages) — **Recadrage** : cadre à poignées, déplacement par l'intérieur, champs L/H/X/Y, ancrage 9 points, verrou ; recadrage hors image signalé (pas d'échec silencieux) | test serveur : recadrage → MediaBox/clip attendus ; découpe rognée au cadre |
| 3.B2 | P1 | ✅ FAIT (7721fdb) — **Échelle** : poignées d'angle (proportionnel) et de côté (libre), chaîne L/H liée/déliée, champ %, largeur ou hauteur imposée | test : L imposée 500 mm → H au prorata ; déliée → déformation exacte |
| 3.B3 | P1 | ✅ FAIT (badge Miroir dans la file ; miroir image seule à éprouver à l'atelier (7721fdb) — **Rotation** 0/90/180/270 et **miroir** au geste ; miroir « image seule » ou « image + découpe » choisi explicitement (⚠️ à éprouver, cf. § 4) | test : miroir image + découpe → tracés miroités ; image seule → tracés inchangés ; colonne « Miroir » dans la file |
| 3.C1 | P1 | **Lés visibles** : traits de lés en pointillés dès que la laize est dépassée, **recouvrement dessiné**, numéro de lé ; frontières déplaçables (lés inégaux) | test : 3 lés inégaux → `mosaique` produit les largeurs saisies ± 0,1 mm |
| 3.C2 | P1 | Tuilage **colonnes × rangées** (lés horizontaux aussi), taille maxi de tuile | test : 2 × 3 tuiles d'une page 3000 × 2000 mm |
| 3.C3 | P2 | **Zone de soudure** sans encre dans le recouvrement (≥ 12,7 mm), coupe au milieu du recouvrement, choix des lés à imprimer (cases) | test : plaques nulles dans la bande ; lé décoché absent de la file |
| 3.C4 | P3 | (Caldera C13) Compléter 3.C : **recouvrement et espace par tuile** (pas seulement global) et **ordre d'impression des tuiles** choisi | test : tuile 2 avec recouvrement 20 mm, les autres 10 → plaques attendues ; ordre 3-1-2 → file dans cet ordre |
| 3.D1 | P1 | ✅ FAIT — **Marques** : traits de coupe, croix de repérage, contour, repères de pli et de couture à distance réglable, style de trait (plein, pointillé, alterné ; longueur, épaisseur, couleur) ; règles d'exclusion (repérage neutralise les traits de coupe) | test : chaque marque à sa cote ± 0,1 mm sur le PDF produit |
| 3.D2 | P1 | 🟡 FAIT sauf alerte « blanc » suspendue : la consigne Kona (5–8 cm de blanc AUTOUR des repères) mesure sans doute la marge support, pas l'écart image–repères ; à reprendre à l'essai Kona — Repères Kona (existants) affichés et déplacés avec l'image dans l'éditeur ; alerte si blanc < 5 cm autour des repères ou image < 25 cm de côté | test : alerte levée sur une page de 200 mm |
| 3.D3 | P2 | ✅ FAIT — **Fond perdu** 4 côtés liés/déliables : couleur, premier pixel, miroir, duplication ; trim box ; repères hors du fond perdu | test : fond perdu miroir 5 mm → pixels miroités au bord |
| 3.D4 | P2 | ✅ FAIT — Œillets par **quantité** ou **espacement**, 4 styles, décalage depuis le trait de coupe | test des deux modes |
| 3.D5 | P2 | ✅ FAIT (copies et réf. commande « — » jusqu'à 5.6) — **Étiquette** imprimée en marge : nom, date/heure, L × H, n° de lé, copies, mode, réf. commande (si fiche, 5.6) | test : texte présent dans le PDF produit |
| 3.D6 | P2 | 🟡 QR FAIT, code-barres 1D à faire (pas de décodeur pour l'éprouver) — (Caldera C5) **QR code / code-barres en marge** avec balises (nom, page, tuile/total, date, id travail, id coupe, machine, réf. commande ERP 5.6), taille, marge, niveau de correction, **contour de découpe autour du code** en option | test : QR du PDF produit décodé = chaîne attendue ; option contour → couche de découpe ajoutée |
| 3.E1 | P2 | ✅ FAIT (50 pas) — Annuler / rétablir (au-delà d'ONYX) | 20 pas, test UI |
| 3.E2 | P3 | ✅ FAIT (sur les plaques CMJN ; BAT sans filtre, signalé) — Filtres couleur par travail (courbes/niveaux par canal, luminosité, saturation), aperçu scindé avant/après, filtre exportable vers un réglage | test : filtre exporté puis appliqué à l'import = même rendu |
| 3.E3 | P3 | ✅ FAIT (sans pipette) — Remplacement de couleur (valeur → valeur, plage) | test sur un aplat |

## Phase 4 — Découpe : couches liées par leur nom (sans matériel, essais 🏭)

Dépend de : 0.3. Débloque : 5 (NestTrimBox, regroupement par découpeuse).

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| 4.1 | P1 | ✅ FAIT — **Préfixes de découpe** configurables (liste séparée par `;`), lus **en début de nom** (`CutContour1` oui, `1CutContour` non), suffixe libre | test : `CutContourKissCut` reconnu, `1CutContour` refusé ; liste actuelle reprise comme défaut |
| 4.2 | P1 | ✅ FAIT — **Une couche par nom de ton** (`CutContour`, `Kisscut`, `Thrucut`, `NestTrimBox`, contour de tuile…) ; **réglage de coupe du même nom appliqué seul**, sinon défaut machine ; modifiable à la main | test : PDF à 2 tons → 2 couches ; réglage « Kisscut » existant appliqué à la couche Kisscut ; HP-GL : un bloc AS/ZF/VS par couche |
| 4.3 | P1 | ✅ FAIT — **Ordre des couches = ordre de coupe**, réordonnable (défaut : mi-chair avant coupe à travers), puis tri existant dans la couche ; activer/désactiver une couche ; mode mono-couche (couper couche par couche) | test : ordre inversé → HP-GL inversé ; couche décochée absente |
| 4.4 | P1 | ✅ FAIT — Changements de couches **enregistrés explicitement** (ou sauvegarde auto assumée) : jamais d'affectation perdue | test : redémarrage → affectations conservées |
| 4.5 | P1 | ✅ FAIT — **Alertes de contrôle préalable** découpe : ton en RVB, tracé avec fond, tracé qui touche la boîte englobante, tracé non au premier plan, contour vectorisé (coupé deux fois), préfixe absent alors qu'une découpeuse est choisie<br>+ Caldera C2 : **tracé de découpe sur calque non imprimable** (OCG print=off) ignoré par défaut avec alerte, option pour le prendre ; alertes « trait non centré » (intérieur/extérieur) et « tracé hors du plan de travail » (fini : 3 PDF de test → 3 alertes ; option activée → le tracé du calque masqué est coupé) | 6 PDF de test → 6 alertes ; un PDF propre → 0 |
| 4.6 | P1 | ✅ FAIT — Fichier de coupe écrit **au début de l'impression**, coupe autorisée seulement à impression terminée à 100 % ; écriture atomique pour les découpeuses par dossier | test : coupe refusée tant que la sortie impression n'est pas `terminee` |
| 4.7 | P1 | 🏭 **Kona Type 3 (MF)** à éprouver : cadre, angles 125 mm, repères intermédiaires, code haut et bas | 🏭 carré de 100 mm coupé sur un MF de 1,5 m, écart mesuré au réglet ≤ 0,5 mm |
| 4.8 | P2 | ✅ FAIT — Rotation du travail de coupe 0/90/180/270 + flèche de sens d'avance dans l'aperçu de coupe | test HP-GL : rotation 90 = coordonnées attendues |
| 4.9 | P2 | ✅ FAIT — Découpe seule (PDF **et SVG**) : 4 rotations, miroir, échelle proportionnelle ou non, **copies X/Y avec écarts**, départ depuis l'origine machine (documenté par le support) | test : SVG 3 × 2 copies, écart 5 mm → 6 contours aux bonnes positions |
| 4.10 | P2 | ✅ FAIT — Contour auto sur bord image / fond perdu / trim box, **choke** (mm) ; « décaler la coupe » vers l'extérieur et « étendre l'image près des tracés » (largeur + seuil), indépendants, avec mention « ne remplace pas l'étalonnage » | test : choke 1 mm → contour réduit de 1 mm ± 0,05 |
| 4.11 | P2 | ✅ FAIT — Tracés ouverts tolérés (option, défaut actif) ; tri des tracés désactivable (ordre de création) | test des deux options |
| 4.12 | P2 | ✅ FAIT — Découpe auto à l'arrivée par machine + politique après coupe (rien / archiver / supprimer), archive restaurable | test des trois politiques |
| 4.13 | P2 | 🏭 Graphtec FC7000 (HP-GL 0,025 / ARMS), Roland VP (CAMM-GL III) | 🏭 carré de 100 mm par machine |
| 4.14 | P3 | ✅ FAIT — Détourage automatique (Make Path) : seuil sur le blanc, suppression des contours intérieurs et des petits, lissage, offset extérieur | test sur un PNG détouré connu |
| 4.15 | P3 | ✅ FAIT — Fond sous repères (bande d'encre ou aplat), compensation de cintrage (largeur, début, flèche), variantes de machine (jeu de repères dupliqué) | tests géométriques |
| 4.16 | P1 | ✅ FAIT — (Caldera C1) **Pré-contrôle de la découpe avant l'impression** : si un travail porte une découpe et que la découpeuse affectée est absente du catalogue, hors ligne, sans connexion valide ou sans réglage pour une couche, la sortie impression est bloquée avec la raison `decoupeuse_indisponible` (jamais d'encre gâchée) | test file : découpeuse supprimée → impression refusée avant envoi ; découpeuse rétablie → part |

## Phase 5 — Imbrication dans la file (sans matériel, essai 🏭)

Dépend de : 1.4 (support chargé), 4.2 (couches, NestTrimBox). Réutilise `imbrication.rs` et `forme.rs`.

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| 5.1 | P1 | ✅ FAIT — **Stratégie de placement par machine** : un par un (défaut), groupé dans l'ordre (sans rotation ni réordonnancement), économie (réimbrication complète à chaque arrivée, rangées ou forme, rotation permise) | test : 5 travaux en économie → planche plus courte qu'en groupé ; ordre conservé en groupé |
| 5.2 | P1 | ✅ FAIT — **Déclencheur** : manuel (« Imprimer maintenant »), temporisé (minutes), à la surface (m² ou % de remplissage d'une longueur cible) | test : seuil 1 m² → départ au travail qui le franchit |
| 5.3 | P1 | ✅ FAIT — **Regroupement** : seuls s'imbriquent les travaux de même support, même mode et même découpeuse (dont « aucune ») ; travaux miroités à l'impression seulement entre eux | test : un travail Kona + un sans découpe → deux planches |
| 5.4 | P1 | ✅ FAIT — Aperçu de l'imbrication en cours dans la file (onglet), compteur de travaux, longueur et % de remplissage | écran : aperçu = PDF produit |
| 5.5 | P1 | ✅ FAIT — Contour de toute la planche (`NestTrimBox`, couche dédiée) ; marges haute/basse pour la table de découpe ; alerte « repères sans tracé de coupe » | test : couche `NestTrimBox` présente dans le HP-GL |
| 5.6 | P1 | ✅ FAIT — **Fiche travail** : n° de commande, client, société, quantité due, échéance (case), notes, opérateur ; échéance = indice de priorité de l'imbrication | champs dans `job.json`, visibles dans la file ; tri par échéance testé |
| 5.7 | P2 | ✅ FAIT — Bande de purge (côté, largeur), justification G/C/D, décalage d'origine, espace entre copies, retournement d'un lé sur deux | tests géométriques |
| 5.8 | P2 | Repères de massicot X/Y (simple/double, épaisseur, blanc image-repère) | quand une Fotoba arrive, sinon P3 |
| 5.9 | P2 | 🏭 Essai : planche d'économie imprimée + découpée Kona | 🏭 planche de 5 travaux sortie et coupée sans intervention entre les deux |
| 5.10 | P3 | Planche manuelle (glisser-déposer, verrou, grille, magnétisme), templates de cellules, données variables (CSV) | quand le besoin est confirmé |
| 5.11 | P2 | ✅ FAIT — (Caldera C3) Compléter 5.2 : déclencheurs **inactivité** (aucun travail reçu depuis X min, distinct du minuteur depuis le 1er travail), **longueur maxi**, **rangée pleine sortie seule** ; chargement **libre** (format figé par la 1re planche) ou **strict** (une planche par format) | test : 3 travaux, inactivité 1 min → départ 1 min après le dernier ; longueur maxi 2 m franchie → départ ; strict + 2 laizes → 2 planches |
| 5.12 | P1 | ✅ FAIT — (demande de Mathieu, 25/10) **Pages et fichiers à ne pas imprimer** : case par page dans la fiche du travail et dans l'imbrication (travail entier ou pages choisies d'un multipage) ; une page décochée n'entre ni dans la planche ni dans la file, et reste réactivable | test : PDF 3 pages, page 2 décochée → planche et file sans la page 2 ; recochée → elle revient |
| 5.13 | P2 | ✅ FAIT — (garde-fou, 25/10) **« Refaire » d'une planche abandonnée** : une planche en échec dont les travaux sont déjà revenus en file ne doit pas pouvoir être relancée (double impression) — refus codé, ou relance qui reprend les travaux de la file | test : planche en échec → travaux revenus → Refaire refusé |

## Phase 6 — Consommation réelle par travail et rapports 🏭

Dépend de : 2.4 (le pilote donne le vrai début et la vraie fin d'une sortie).

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| 6.1 | P1 | ✅ FAIT — **Attribution par fenêtre** : consommation d'une sortie = `releves::consommation(debut, fin)` de sa machine (relevé forcé au début et à la fin, en plus de la minute) ; rangée dans la sortie | test : relevés simulés → ml par canal attribués à la bonne sortie ; deux sorties consécutives sans chevauchement |
| 6.2 | P1 | ✅ FAIT — **Résolution de la mesure** : l'encre est au % près (1 % d'une poche de 700 ml = 7 ml) → afficher « < seuil » plutôt qu'un 0 ; attribution au prorata de la couverture estimée quand la baisse couvre plusieurs travaux | test : petit travail → « < 7 ml », pas 0 |
| 6.3 | P1 | 🟡 CODE FAIT, essai réel 🏭 à faire — Longueur de rouleau réelle par sortie (prtInputCurrentLevel) dont **gâche** (amorce, marges : +204 mm observés) | 🏭 travail de 369,8 mm → 574 mm relevés, gâche 204 mm affichée |
| 6.4 | P1 | ✅ FAIT — **Journal des impressions** : date, machine, support, mode, opérateur, surface, longueur, encre réelle et estimée ; rétention réglable (défaut 90 j), purge manuelle | test : rétention 1 j → entrées anciennes purgées, travaux non touchés |
| 6.5 | P1 | ✅ FAIT — **Rapports CSV** : réel par travail et par période (séparateur `;`, décimales FR), estimé vs réel côte à côte | test : CSV ouvert dans LibreOffice, colonnes stables ; total = somme des lignes |
| 6.6 | P2 | ✅ FAIT — **Facteur de correction** par machine × résolution × encre (défaut 1), proposé = réel / estimé sur N travaux, appliqué seulement sur validation | test : facteur proposé = ratio attendu sur données simulées |
| 6.7 | P2 | ✅ FAIT — Coût encre « non calculable » affiché plutôt qu'un faux chiffre quand le mode n'est pas mesurable | test |
| 6.8 | P2 | **Étiquettes Zebra 100 × 150 mm** (ZPL, Zebra partagée par CUPS sur le serveur) : les TROIS usages confirmés le 24/10 — travail fini (réf. commande, client, dimensions), expédition, rouleau en stock | 🏭 une étiquette imprimée et lisible (code-barres scanné) |
| 6.9 | P1 | ✅ FAIT — (Caldera C0) **Encre estimée par comptage de points** (après 2.3) : ml par canal = Σ points de chaque taille × volume de goutte (pL) du mode ; volumes éditables par mode (défauts saisis par nous) ; remplace l'estimation sur l'aperçu dès que le mode est tramé ; reste « non calculable » en contone | test : bande synthétique de N points de chaque taille → ml attendu au µl près ; travail réel : écart estimé/SNMP affiché à côté (6.5) |

## Phase 7 — Pont ERP ↔ RIP (côté RIP)

Dépend de : 1.1 (catalogue avec modes), 0.5 ; 7.4 dépend de 6.1. Les montants ne transitent jamais : le RIP envoie des
quantités physiques, l'ERP valorise. Le RIP **sort** vers l'ERP (aucun port entrant ouvert sur l'atelier).

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| 7.1 | P1 | **Accès machine à machine** : jeton d'API propre à l'ERP (portée limitée), exception ACCEPTÉE par le gérant le 24/10 ; le RIP sort vers l'ERP, aucun port ouvert à l'atelier | test : jeton valide → 200 sur les routes ERP, 403 ailleurs ; sans jeton → `client_requis` inchangé |
| 7.2 | P1 | ✅ FAIT côté RIP (à éprouver contre le vrai ERP quand PT1 sera en production) — **Catalogue machines** publié : machines, supports, modes, réglages, découpeuses, avec id stables ; poussé au démarrage, à chaque modification et sur demande | test : modifier un mode → événement `catalogue.modifie` émis |
| 7.3 | P1 | ✅ FAIT côté RIP le 28/10 (lot R3.1 : fichier téléchargé par URL signée, SHA-256 vérifié, travail créé sous l'uuid de l'envoi, refus codés) — **Réception d'un travail** : `erp_job_ref` + version, fichier par URL signée avec SHA-256 vérifié, dimensions finies, fond perdu, quantité, machine/support/mode/réglage (par id), couches blanc/argent, découpe, notes ; combinaison inexistante **refusée** avec message ; renvoi identique = pas de doublon | tests : SHA faux → refus ; même ref + même version → même travail ; version +1 → nouveau travail, l'ancien signalé |
| 7.4 | P1 | ✅ FAIT côté RIP (à éprouver contre le vrai ERP quand PT1 sera en production) — **Statut par événement** : reçu, en attente, ripé, prêt, en impression, imprimé, en découpe, découpé, erreur (motif), annulé ; webhook HMAC, numéro de séquence, relecture « tout depuis N » | test : coupure réseau simulée → l'ERP récupère les événements manquants par séquence |
| 7.5 | P1 | ✅ FAIT côté RIP (à éprouver contre le vrai ERP quand PT1 sera en production) — **Consommation réelle** remontée en fin d'impression puis de découpe : m², longueur dont gâche, ml par canal, temps machine réel, copies sorties, machine/support/mode réellement utilisés | test de bout en bout avec un ERP simulé |
| 7.6 | P2 | ✅ FAIT côté RIP le 28/10 sur les travaux reçus et sur un fichier avant envoi, rendu caché pour un devis (lot R4, `estimations` / `travail.estimation`, sans montant) — Aperçu ripé + estimation synchrone (surface, longueur après imbrication, lés, durée, encre estimée par canal) | test : estimation = valeurs de la fiche du travail une fois créé |
| 7.7 | P2 | ✅ FAIT côté RIP le 28/10 (lot R3.2, `actions` / `travail.action`) — Annulation, mise en attente, priorité, échéance depuis l'ERP ; un travail imprimé ne s'annule pas, il se signale | test |
| 7.8 | P3 | Travaux liés (groupe, ordre des parties), inventaire média, opérateur affecté, journal machine, réimpression depuis l'ERP | au besoin |
| 7.9 | P2 | ✅ FAIT côté RIP le 25/10 (`machine.etat` ; bac d'encre usagée `null` tant qu'il n'est pas lu sur l'Epson 🏭 ; envoi à la demande : à convenir avec l'ERP) — (Caldera C18) Compléter 7.2/7.4 : **état machine publié à l'ERP** (support chargé, niveaux d'encre, **bac d'encre usagée**, chauffages, alertes) par événement et à la demande | test : relevé SNMP enregistré → événement `machine.etat` avec les mêmes valeurs |

## Phase 8 — Couleur mesurée 🏭 (spectro : i1Pro disponible, confirmé le 24/10 ; sous Linux par ArgyllCMS)

Dépend de : 2.2, 2.3 (séparation et tramage), 1.1 (le mode porte les étapes).

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| 8.1 | P1 | Restriction d'encre par canal (mire maison, lecture spectro, méthode du « crochet de chroma » ; volume linéaire pour WH/MS) | 🏭 restriction enregistrée dans un mode, mire relue |
| 8.2 | P1 | Linéarisation 1D par canal vers des densités cibles (N-factor, variance, lissage, gain de point), historique conservé (ONYX écrase) | 🏭 rampe mesurée linéaire à ± 2 % |
| 8.3 | P1 | Limites d'encre combinées (simples, doubles, neutres + Or/Rd), encre totale, noir hors limite | 🏭 aplat 300 % sans bavure après séchage |
| 8.4 | P1 | Profil ICC mesuré (moteur libre, CGATS en entrée/sortie), aucun profil ONYX | 🏭 ΔE2000 moyen ≤ 2 sur une mire de contrôle |
| 8.5 | P1 | ✅ FAIT le 31/10 côté serveur (recalibration.rs, par imprimante : rappel 14 j, périmé à 90 j, poche changée = hausse SNMP ≥ 10 points, nouveau lot / maintenance saisis ; routes `recalibration`, `etalonnage`, `/api/recalibrations` ; jamais bloquant ; affichage dans les clients natifs à faire ; par mode quand 8.1–8.4 existeront) — Recalibration : rappel 14 j, plafond 3 mois, déclencheurs (nouveau lot, maintenance, changement de poche détecté par SNMP) | test : hausse SNMP d'une encre → rappel |
| 8.6 | P2 | ✅ FAIT le 25/10 sauf ΔE estimé (→ ArgyllCMS, 8.4) et priorité par mode / alerte PDF sans vrai ton direct (→ 8.7) — Table de tons directs (Lab, alias, casse **et** espaces exacts), remplacement par mode prioritaire, journal des noms non trouvés, ΔE estimé, alerte PDF sans vrai ton direct<br>+ Caldera C11 : **import d'une bibliothèque Lab de fournisseur** au format ouvert (CxF, CGATS ou CSV Lab) — ex. nuancier HEXIS ; jamais de bibliothèque Pantone sans licence ; profil d'entrée CMJN par défaut choisi et documenté (fini : import CSV de 50 teintes → 50 tons directs ; doublon de nom refusé ou renommé) | tests sur PDF de référence |
| 8.7 | P2 | 🟡 Intention FAITE le 25/10 (serveur → réglage → travail ; test « seuls les travaux par défaut changent ») ; recette 9 encres 🏭 (mesures i1Pro) — Intention propre aux tons directs (globale → réglage → travail), recherche de recette 9 encres (≈ Smart Spot Search) | test : intention globale changée → seuls les travaux « par défaut » changent |
| 8.8 | P2 | Nuanciers 1D/2D/3D, statut « en attente de mesure », « ajouter au mode » | 🏭 |
| 8.9 | P2 | Contrôle de constance (référence par mode, ΔE2000 moyen/max/95e, expiration, rapport HTML), jamais bloquant | 🏭 |
| 8.10 | P3 | G7 (garder la meilleure itération), DeviceLink auto, profils hybrides spectraux, multicouche générique | au besoin |
| 8.11 | P2 | ✅ FAIT le 26/10 (separation.rs `Bascule` douce / équilibrée / franche portée par le mode avec son encre totale ; linearisation.rs : limite proposée, mire CGATS 8 plages < 20 % sur 21) ; essai papier des trois bascules 🏭 — (Caldera C10) Compléter 2.2 / 8.3 : **préréglages de séparation des claires** (3 profils de bascule nommés par nous), **limite d'encre totale proposée** à partir de la linéarisation mesurée (validée à la main), cibles de linéarisation **plus denses en hautes lumières** | test : préréglage appliqué → courbes attendues ; mesure simulée → limite proposée reproductible ; mire générée avec plus de plages < 20 % |
| 8.12 | P3 | (Caldera C17) **Économie d'encre** par GCR (remplacement du gris par le noir) appliqué à un profil existant, niveaux réglables, départ du noir réglable — via le moteur libre (ArgyllCMS), jamais un module tiers | 🏭 ΔE2000 moyen ≤ 2 entre profil d'origine et profil économe sur la mire, encre totale réduite et mesurée |

## Phase 9 — Reste, au fil de l'eau

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| 9.1 | P1 | ✅ FAIT le 26/10 (regles.rs : dossier `entrees-auto/`, motif `*`/`?` et petit/grand côté de la 1re page PDF → réglage, éditeur dans Système, sauvegarde) — Règles de dossiers surveillés (préfixe, taille → réglage) — « R » du tableau de bord | test : `BACHE_*.pdf` → réglage Bâche |
| 9.2 | **P1** | ✅ FAIT le 25/10 (images → PDF en flux, taille réelle, CMJN/RVB/gris 8-16 bits, alpha en masque, ICC, JPEG tel quel ; 8 Go max) ; canaux de tons directs du TIFF FAITS le 26/10 (plaque par canal en surimpression, noms Photoshop 1045/1006, sélections 1077/1007 ignorées, `Spot1`/`White`/`Blanc` → White) ; sens des valeurs à confirmer sur un vrai TIFF Photoshop 🏭 — Formats d'entrée TIFF (8/16 bits, CMJN + spots), JPEG, PNG — confirmé le 24/10 (reçus en plus du PDF) | test : TIFF CMJN + canal `Spot1` → plaque blanc |
| 9.3 | P2 | ✅ FAIT le 26/10 (cases `page_par_travail` et `abandon_polices` du réglage : pages 2 à n en travaux « (p. k/n) », refus `polices_manquantes` hors 14 polices de base) — Une page par travail (case du réglage) ; abandon si polices manquantes (option du réglage) | tests |
| 9.4 | P2 | ✅ FAIT le 26/10 (ports.rs : connexions `ipp` (Print-Job brut, flux par morceaux, aperçu de l'URI) et `rip_eko` (route `/api/renvoi/{machine}` à jeton) ; essai sur un vrai CUPS, file raw) — Port IPP (hôte + chemin, aperçu de l'URI) ; port « renvoi vers un autre RIP E-KO » | test sur un serveur IPP de test (CUPS) |
| 9.5 | P2 | ✅ FAIT le 26/10 (champ `epreuve` de la mise en file : copie « -BAT » liée par `bat_de`, original `Suspendue`, liste « BAT sur… » de la page) — Épreuve sur une 2e machine (copie « -BAT », original en attente) | test file |
| 9.6 | P2 | ✅ FAIT le 28/10 (`poids` par sortie dans `/api/file`, `tampon` dans `/api/systeme`, colonne « Tampon » et ligne disque sous le bandeau des machines) — Compteurs de file : poids du tampon, % disque | écran |
| 9.7 | P3 | ✅ FAIT le 28/10 (`glossaire.md` / `glossary.md`, `machines.md` FR/EN, « Exigences du poste serveur » dans l'installation) — Glossaire FR/EN, aide par machine, exigences poste dans la doc (SSD dédié, onduleur, IP fixes) | docs publiées |
| 9.8 | P3 | Tableau de bord type Hub (live, historique) — dans l'ERP plutôt que dans le RIP | décision ERP |
| 9.9 | P3 | Correspondance matière Zünd, rouleau à rouleau, massicot Fotoba<br>+ Caldera C7 : **association support → matière de découpe** par motif (nom, id, type) pour les découpeuses qui ont des matières (Zünd, CAMM-GL avec presets) ; non associé → défaut + alerte (fini : « HEXIS V3000 Vinyle » → matière Vinyle ; support non reconnu → défaut signalé) | si la machine arrive |
| 9.10 | P2 | ✅ FAIT le 28/10 (balises.rs : motif de `/api/balises`, alertes `nom_non_conforme` / `balise_invalide` / `balise_sans_effet` / `balise_proportions`, `imprimante` et `repetition` de la demande, éditeur dans Système, sauvegarde) — (Caldera C4) Étendre 9.1 : **motif de nom de fichier à balises** (copies, machine, réglage, support, laize, échelle, largeur, hauteur, répétition) qui pré-remplit le travail ; double séparateur conseillé ; journal d'extraction en cas d'échec | test : `Affiche__epson__bache__30.pdf` → 30 copies, réglage Bâche ; nom non conforme → travail importé avec alerte, pas d'échec silencieux |
| 9.11 | P2 | ✅ FAIT le 28/10 (rendus.rs : `EKO_RIP_RENDUS` rendus simultanés, un par cœur et 4 au plus ; attente par ordre d'arrivée, manuel d'abord ; sortie en file des arrivées automatiques dans l'ordre de dépôt ; `rendus` dans `/api/systeme`) — (Caldera C6) **Rendu parallèle** des arrivées (dossiers et API) sur N travaux simultanés, **ordre de sortie = ordre d'arrivée**, import manuel prioritaire sur l'automatique | test : 20 fichiers déposés, 4 rendus parallèles → file dans l'ordre de dépôt ; un import manuel passe devant |
| 9.16 | **P1** | ✅ FAIT le 28/10 (render.rs : au-delà de 3 Go de composite, page rendue par bandes d'1 Go (MediaBox par bande sur la grille de points), plaques recollées en LZW, BigTIFF au-delà de 3,5 Go ; identique point pour point au rendu d'un trait ; bâche 1,6 × 3 m à 720 dpi rendue en 6 min sur 4 cœurs, essai `bache_1600_x_3000_a_720_dpi` ignoré par défaut) — **Rendu par bandes des très grands formats** : `tiffsep` écrit son composite CMJN en TIFF classique (≤ 4 Go, `UseBigTIFF` ignoré par ce périphérique, mesuré le 25/10 sur gs 10.08) ; au-delà (ex. bâche 1,6 × 3 m à 720 dpi ≈ 15 Go), le rendu échoue. Rendre par bandes (`-dFirstLine`/page découpée par CropBox) puis recoller les plaques, ou produire les plaques autrement | test : page 1,6 × 3 m à 720 dpi rendue, plaques complètes, aucune au-delà de 4 Go en TIFF classique |
| 9.12 | P2 | ✅ FAIT le 28/10 (réglage `blanc.alpha` : `aucune` / `blanc` / `vernis` ; images.rs pose une plaque `White` ou `Varnish` = canal alpha, sans masque, en surimpression après les tons directs ; un canal TIFF du même nom passe avant) — (Caldera C8) Étendre 9.2 : **canal alpha (PNG/TIFF) → plaque blanc ou vernis** au choix du réglage | test : PNG détouré → plaque blanc = masque alpha ± 1 px |
| 9.13 | P3 | ✅ FAIT le 28/10 (epreuve.rs : TIFF des plaques en flux, CMJN + profil du support ou RVB par Ghostscript, tons en canaux Photoshop 1006/1007 / aplatis / supprimés, réduction k × k à taille physique exacte sous `max_mo` ; route `epreuve.tif`, réglages `epreuve` et `epreuve_seule`) — (Caldera C12) **Épreuve TIFF** après RIP : colorimétrique (ICC) ou valeurs machine, tons directs conservés en canaux / aplatis / supprimés, option « épreuve seule », plafond de taille ; sert aussi à **certifier les cotes** d'un lé | test : TIFF produit = dimensions physiques ± 0,5 mm ; canal blanc présent en mode « conserver » |
| 9.14 | P3 | ⏸ REPORTÉ le 30/10 aux clients natifs (refonte SwiftUI / GTK 4) : inutile de l'ajouter au client Tauri qu'ils remplacent — (Caldera C15) **Client connecté à plusieurs serveurs RIP** (atelier multi-sites ou 2e serveur), file unifiée dans le client | test : client Tauri avec 2 serveurs de test → les deux files visibles, envoi vers chacun |
| 9.15 | P3 | ✅ FAIT le 30/10 (réglage `reechantillonnage` : `proche_voisin` par défaut ou `lisse` = `-dDOINTERPOLATE` de Ghostscript, filtre cubique ; mire 100 ppi ×4 : 48,6 dB contre 30,2 dB) — (Caldera C16) **Rééchantillonnage bicubique** (ou meilleur) à l'agrandissement des images matricielles, choix par réglage | test : image 100 ppi ×4 → PSNR meilleur que plus proche voisin sur une mire |

## Phase 10 — Clients natifs (ajoutée le 31/10, suite de la refonte du 30/10)

Les clients Mac (SwiftUI) et Linux (GTK 4 / libadwaita) remplacent le client Tauri ; cahier
dans `docs/clients-natifs.md`. Le lot 1 (connexion, travaux, inspecteur, envoi, machines,
système, notifications) est livré dans la 0.29.0. Le serveur expose déjà tout ce qui suit.

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| 10.1 | P1 | **Écrans d'administration natifs** (lot 2) : supports et modes, règles et balises, tarifs, tons directs, pont ERP, sauvegarde, éditeur de machines ; aujourd'hui ouverts dans une fenêtre web par « Administration… » | chaque écran du menu « Administration… » existe en natif sur Mac et Linux ; la fenêtre web n'est plus nécessaire |
| 10.2 | P2 | **Réglages récents dans les clients** : lissage des images agrandies (`reechantillonnage`, 9.15), empilement de couches (`empilement`, 2.10), badge de recalibration par machine (`/api/recalibrations`, 8.5) | un travail envoyé avec `lisse` et un empilement blanc-couleur-blanc depuis chaque client ; badge « Recalibration conseillée / périmée » visible sur la carte machine |
| 10.3 | P3 | **Plusieurs serveurs RIP dans un client** (reprend 9.14) : file unifiée, envoi vers le serveur choisi | client relié à 2 serveurs de test → les deux files visibles, envoi vers chacun |
| 10.4 | P2 | **Client Windows** (WinUI 3, même API), quand un PC Windows est disponible pour le construire et le signer | installateur signé, lot 1 complet sur Windows 11 |
| 10.5 | P3 | **Planche d'imbrication, détourage et mise en coupe en natif** (éditeurs visuels de la phase 3 et 5) | ouverts et utilisables sans la fenêtre web |

## Décompte

| Phase | P1 | P2 | P3 | Total |
|---|---|---|---|---|
| 0 Fiabilité | 4 | 3 | 1 | 8 |
| 1 Supports / modes | 5 | 2 | 1 | 8 |
| 2 Pilote Epson | 6 | 3 | 1 | 10 |
| 3 Éditeur visuel | 10 | 6 | 3 | 19 |
| 4 Découpe | 8 | 6 | 2 | 16 |
| 5 Imbrication en file | 7 | 5 | 1 | 13 |
| 6 Conso réelle / rapports | 6 | 3 | 0 | 9 |
| 7 Pont ERP | 5 | 3 | 1 | 9 |
| 8 Couleur mesurée | 5 | 5 | 2 | 12 |
| 9 Reste | 2 | 7 | 6 | 15 |
| 10 Clients natifs | 1 | 2 | 2 | 5 |
| **Total** | **59** | **45** | **20** | **124** |

## Les 5 premiers items

1. **0.1** Résistance aux coupures : aujourd'hui un `file.json` illisible donne une file vide sans alerte.
2. **0.3** Tests de non-régression géométrie (protègent tout ce qui suit).
3. **1.1** Modèle Support → Mode (prérequis du pilote, de l'imbrication et du pont ERP).
4. **2.1** Émetteur ESC/P2 validé contre la structure de la capture, sans machine.
5. **3.A1** Aperçu grand format de l'éditeur (zoom, loupe, règles, laize) : ouvre toute la phase 3.

---

# 3. Questions au gérant (seulement ce qui bloque)

**RÉPONSES du 24/10/2026** : (1) Zebra : les trois usages (travail fini, expédition, rouleau) ;
(2) premier essai Epson : comme la capture (même mode, même support) ; (3) jeton ERP : oui ;
(4) spectro : i1Pro ; (5) TIFF, JPEG et PNG reçus en plus du PDF → 9.2 passe en P1.
Questions ci-dessous conservées pour mémoire.

1. **Zebra 100 × 150 mm** : pour quoi exactement ? (a) étiquette à coller sur le travail fini (réf. commande, client,
   dimensions), (b) étiquette d'expédition, (c) étiquette de rouleau en stock. Et quel modèle, branché comment
   (réseau ou USB) ? Bloque 6.8.
2. **Premier essai Epson** : quel mode (le 720 × 720 8 passes de la capture ?) et quel support (HEXIS V3000WG ?) pour
   le premier aplat ? Bloque 2.3–2.4.
3. **Pont ERP** : accepte-t-on une exception à « interface et API réservées aux clients » sous forme d'un jeton
   d'API propre à l'ERP, le RIP sortant vers l'ERP (webhooks) sans port ouvert à l'atelier ? Bloque 7.1.
4. **Spectro** : lequel est disponible (l'i1Pro 2 est en fin de vie, pilote bloqué par Windows 11 25H2 ; sous Linux,
   ArgyllCMS) ? Bloque toute la phase 8.
5. **Formats d'entrée** : recevez-vous des TIFF, JPEG ou PNG à imprimer directement, ou tout arrive-t-il en PDF ?
   Décide si 9.2 passe en P1.

# 4. À éprouver à l'atelier

| Point | Pourquoi | Item |
|---|---|---|
| **Mirror / Reflect at Print Time et les tracés de coupe** | le support se contredit (fiche ancienne : aucun ne retourne les tracés, passer par « Mirror Cut Paths » ; fiche 25.5 et notes de version : Mirror les retourne) ; Mirrored fiable en découpe seulement avec un seul travail dans l'imbrication | 3.B3, 5.3 |
| Kona Type 3 (MF) | cadre, angles 125 mm, repères intermédiaires jamais lus | 4.7 |
| Premier flux ESC/P2 sur l'Epson | structure validée hors ligne, pas encore sur papier | 2.4 |
| Canaux 7 et 10 (Or / Rd) ; canaux WH / MS | attribution déduite, capture blanc/argent à faire | 2.1, 2.5 |
| Consommation SNMP vs réalité | encre au % près ; gâche +204 mm observée une fois | 6.2, 6.3, 6.6 |
| Décalage de découpe mesuré | réglage existant, jamais mesuré au réglet | 4.7 |
| Règle des repères : 5–8 cm de blanc, image ≥ 25 cm | consigne du support (repères non lus), à confirmer sur le Kona | 3.D2 |
| Couche Kona « V3000-Blanc » | laissée vide (décision du 24/10) | 4.2 |
| Graphtec FC7000, Roland VP (découpe et impression) | protocoles non éprouvés | 4.13 |
| Cadences réelles par mode | durée estimée sur fiche constructeur | 2.7 |
| Rotation auto en résolution non carrée | bogue ONYX documenté, à reproduire en test puis sur machine | 0.3, 2.7 |

# 5. Contradictions et écarts restants

- `job-editor.md` § 4.4 recommande « Reflect at Print Time » sans parler des tracés de coupe ; non corrigé (hors
  périmètre de cette passe), à lire avec le point « à éprouver » ci-dessus.
- `serveur/src/epson.rs` (en-tête) décrit encore le mode contone 5 canaux ; la capture du 24/10 montre 9 canaux tramés
  en 2 bits. Le code n'est pas touché ici : l'item 2.6 le remplace.
- `todo-rip-multiplateforme.md` (tableau de bord) annonce « publié v0.1.0 → v0.10.0, prochaine v0.11.0 », alors que le
  CHANGELOG va jusqu'à **0.13.0** : tableau de bord à rafraîchir.
- Dossiers chauds : ONYX conseille de **copier** les fichiers dans le dossier ; le nôtre **déplace** le fichier
  traité vers `traites/` (`main.rs:1411`). Pas une contradiction de doc, mais un choix à garder ou changer (9.1).
- Calcul d'encre : ONYX ne le fait qu'en demi-ton ; notre estimation (`couverture.rs`) se fait sur l'aperçu quel que
  soit le mode. Sans effet tant que le pilote Epson est en demi-ton (2.3).
