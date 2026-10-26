# CalderaRIP — synthèse documentaire pour E-KO RIP Center

Date : 2026-10-24. Recherche en lecture seule, sources publiques uniquement (caldera.com, helpdesk.caldera.com,
guide HEXIS, presse spécialisée). Aucun binaire téléchargé, aucun formulaire soumis, aucune connexion.
Tout est reformulé. Version courante observée : **CalderaRIP V19.3.0 (août 2026)**.

Rapport aux items de `tasks/roadmap-rip-onyx.md` : **Oui** = item existant qui couvre ; **Partiel** = item proche à
compléter ; **Non** = rien dans la roadmap.

---

## 1. Architecture

### 1.1 Postes, OS, interface

| Sujet | Ce que fait Caldera | Source |
|---|---|---|
| Modèle | Client/serveur sur le réseau local. Un poste peut être **client et serveur à la fois**. Un client envoie des travaux à un ou plusieurs serveurs ; chaque serveur pilote ses machines (impression et découpe). | helpdesk 21882880683793 |
| Licence partagée | Une licence principale partagée entre tous les postes du réseau ; le poste qui porte la licence (« KeyServer ») démarre en premier, puis les autres serveurs. | idem |
| Rendu partagé | Plusieurs serveurs peuvent se répartir le rendu de travaux identiques (calcul parallèle). | idem |
| Réseau | Communication RPC par la **première interface Ethernet** (eth0 / en0) ; mauvaise interface = échecs RPC. Port du service RIP : **45344** (passé de 45343 en V18) ; port du serveur principal **12340**. Service distant ajouté par IP. | helpdesk 21882880683793, 30164546973969 |
| Reprise sur panne | Conseil : installer tous les pilotes sur tous les serveurs ; si un poste tombe, on change quelques IP. Les profils média doivent être présents sur client **et** serveur. | 21882880683793 |
| OS (V19) | **Ubuntu 22.04 / 24.04** ; macOS Sonoma → Tahoe 26. Debian abandonné en V18 (Buster était la dernière). Pas de Windows, sauf le serveur de licences (virtualisation) depuis V12. | comparatif des versions ; changelog V18.0 |
| Interface | Application de bureau ; sur macOS l'interface passe par **XQuartz (X11)** (cf. problèmes de rafraîchissement Tahoe). Aucune interface web d'exploitation trouvée. | changelog V19 |
| Utilisateurs | 1 « interface graphique » (utilisateur) incluse, utilisateurs supplémentaires vendus ; message d'erreur dédié quand la limite est atteinte (19.3.0). | 21882880683793, changelog V19 |
| Outil d'administration | **CalderaDock** = boîte à outils : installation, licences numériques (sans dongle depuis V12), mises à jour, téléchargement des profils (remplace la Profile Library depuis V12), instantanés, nettoyage des temporaires. **Ce n'est pas un connecteur MIS.** KeyManager retiré en V18. | comparatif ; changelog V18.0 ; EasyMedia overview |

### 1.2 Moteur de rendu (et ce qui reste de Ghostscript)

| Version | PDF | PS / EPS |
|---|---|---|
| 2009 | 1re solution **grand format** à intégrer l'Adobe PDF Print Engine (APPE) ; rendu PDF natif sans passer par PostScript (prix DPI au SGIA 09) | — |
| V11 → V13 | APPE 4 puis 5 | **Ghostscript natif** |
| V14 → V18 | APPE 5.5 (V14, ajoute un rendu des traits fins), 6 (V16), 6.2 (V17), 6.3 (V18) | **PS2PDF** (conversion PS → PDF) |
| V19.0.0 (nov. 2025) | **APPE 7**, puis 7.1 en 19.2.0 ; **processeur AVX2 obligatoire** ; FileMan ~20 % plus rapide | **Ghostscript retiré** : les fichiers importés jadis par GS doivent être réimportés ; APPE seul interpréteur |

Conclusion : depuis V19, plus rien de Ghostscript dans le produit ; avant V19, GS servait au PostScript (directement
jusqu'à V13, puis via conversion).

### 1.3 Files, dossiers surveillés, automatisation

| Brique | Rôle |
|---|---|
| **FileMan** | Import et analyse des fichiers (paramètres de RIP, liste des noms de découpe, option « erreur si substitution de police », extraction de métadonnées du nom). |
| **Print Module** | Fenêtre d'une machine : support, qualité, couleur, mise en page (Page Setup : marques, fond perdu, massicots, annotations, codes optiques, œillets, bande de contrôle, barre de couleurs, découpe). Actions : Print, RIP, RIP puis Print, **Nest**. |
| **Spooler** | File par machine en 3 zones : **en cours** (rendu, impression, attente, imbriqué, bloqué), **terminés** (avec copies demandées / faites, découpe générée), **en erreur** (message). Actions : priorité, abandon, purge, bloquer, imprimer, réglages, **rouvrir**, **copier / déplacer vers la file d'une autre machine, même sur un autre poste** (administrateur), aperçu, export d'aperçu. Onglet CostProof. |
| **CalderaJobs** (V13) | Retrouver, réimprimer, archiver, supprimer tout travail ; filtre par date (V16). |
| **QuickPrint** | Préréglage utilisé comme flux (≈ Quick Set) ; les réglages de découpe sont rattachés au préréglage depuis 19.0.2. |
| **HotFolders** | Dossiers illimités, scrutés ; chargent dans un onglet de la barre d'images ou envoient à un **Workflow**. Depuis V14 : **traitement parallèle** (annoncé +35 % PDF, +60 % TIFF), ordre d'entrée conservé si tout se passe bien, **envoi manuel toujours prioritaire**. Post-traitement (supprimer ou conserver), menu « ordre de traitement », duplication (V18), **export/import de configuration entre postes** (V18.1, CalderaCare). |
| **SmartImport** | Lit les réglages dans le **nom du fichier** par motif à balises : copies, machine, préréglage, support, chargement (rouleau/format), échelle, largeur, hauteur, répétition verticale (textile). Conseil : double séparateur `__` pour éviter les conflits. CalderaCare / abonnement. |
| **Nest-O-Matik** | Imbrication automatique (voir § 2.3). |
| **Disque** | Depuis V18.1 : avertissement à **10 Go** libres avant envoi, refus sous **5 Go** ; en automatique, pas d'avertissement mais travail marqué « abandonné » en file d'erreur. |

### 1.4 Licences

- Abonnement annuel ou perpétuel, trois paliers **Essentials / Pro / Max** : même fonctions partout, le palier dépend
  de la **catégorie de pilotes** (taille/type de machine).
- **Licence par pilote d'imprimante** : 1 pilote inclus, pilotes supplémentaires annoncés « illimités » (achetés en
  plus, prix non publié) ; changement de pilote illimité dans la même catégorie.
- Pilote de **découpeuse** sous licence propre (V18 : erreur affichée **avant** l'impression si la licence manque).
- Pilotes « -ML » (multicouche) gratuits pour qui a le pilote standard, **un seul utilisable à la fois**.
- **CalderaCare** (maintenance) débloque : SmartImport, PrintProof, REST API, Smart Materials, export de dossiers
  surveillés, multicouche avancé par dossier surveillé, Print Standard Verifier, accélération matérielle.

### 1.5 Connecteurs MIS / ERP

| Connecteur | Principe | Accès |
|---|---|---|
| **Nexio** (ex-« Bridge ») | JDF/JMF : soumission de travaux et collecte de rapports ; un module par application tierce, instance dans un « JobTracker » (voir, trier, lancer, arrêter, erreurs). Versions complète (submit + collect) ou submit seul. Configurateur Enfocus Switch. Les balises Nexio (client, n° client…) sont utilisables dans les annotations et QR codes. | Option, souvent avec développement sur devis ; inclus dans les abonnements et CalderaCare depuis nov. 2025 (selon un résultat de recherche, non revérifié) |
| **CalderaConnect** (REST API) | JSON sur HTTP, définition **OpenAPI**, SDK. V1 (~V17), **V2.0 (nov. 2024)**, V2.1 (avr. 2025, exige V18.1). Routes vues : `POST /devices/{device}/jobs` (soumission), `/jobs`, `GET /devices/{device}/information` (support chargé, cartouches, **bac d'encre usagée**, chauffages — sauf HP, Canon Pro, Mutoh, pilotes « Grand »). **Identifiants stables** depuis V2, cache des machines si un service tombe, multi-sites. Option V2.1 : ne pas conserver les travaux soumis par l'API. | CalderaCare ou GrandRIP MAX |
| **PrimeCenter** | Logiciel de prépresse **en amont** (imbrication vraie forme, tuilage, fond perdu, contours, repères, QR, recto-verso, découpage en lots, rapport PDF de mise en page). Ne pilote aucune machine. V4.4 + RIP V19 : fichiers « Print&Cut-Ready » multicouches lus directement. | Produit séparé (« PrimeRIP Bundle ») |
| CalderaDock | Administration (§ 1.1), pas un connecteur. | — |

---

## 2. Fonctions par domaine, et couverture par notre roadmap

### 2.1 File / Queue

| Fonction Caldera | Roadmap |
|---|---|
| File par machine, priorité, abandon, blocage, réimpression (« rouvrir ») | Oui — Fait (matrice 1.1) |
| 3 zones en cours / terminés / en erreur avec message | Oui — 0.4 (raison d'attente), file existante |
| Copies demandées / copies faites dans la file | Partiel — 5.6 (quantité due) ; pas de compteur « faites » |
| Copier / **déplacer** un travail vers une autre machine, **y compris sur un autre serveur** | Partiel — matrice « copier vers une autre machine », 9.4 (renvoi vers un autre RIP) |
| Dossiers surveillés illimités → flux | Oui — dossier chaud par réglage + 9.1 |
| Traitement parallèle des dossiers, ordre conservé, envoi manuel prioritaire | **Non** → C6 |
| Import par motif dans le nom de fichier (SmartImport) | Partiel — 9.1 (préfixe → réglage seulement) → C4 |
| Export/import de configuration de dossiers surveillés | Oui — 1.6 (réglages choisis ; nos dossiers sont liés aux réglages) |
| Seuils disque 10 Go (avertir) / 5 Go (refuser), travail auto marqué abandonné | Partiel — 0.6 (un seuil + refus) → C14 |
| Recherche / archive / filtre par date (CalderaJobs) | Partiel — recherche + filtres (matrice 1.1) ; pas de filtre de date explicite |
| Pool / client multi-serveurs | Partiel — 1.7 (pool) ; un client sur N serveurs non décrit → C15 |

### 2.2 Éditeur de travail (Print Module / Page Setup)

| Fonction Caldera | Roadmap |
|---|---|
| Échelle, rotation, miroir, recadrage | Oui — 3.B1, 3.B2, 3.B3 |
| Rééchantillonnage **bicubique** à l'agrandissement | **Non** → C16 |
| Fond perdu par clonage de pixels / duplication de contour (PrintBleed) | Oui — 3.D3 |
| Repères de coupe, barre de couleurs, bande de contrôle, œillets | Oui — 3.D1, 3.D4 ; barre/bande de contrôle : Partiel (8.9 contrôle de constance, pas de bande imprimée) |
| Annotations (texte en marge, balises) | Oui — 3.D5 |
| **QR code / code optique** en marge avec balises (fichier, page, tuile, nb de tuiles, date, id travail, id coupe, machine, commentaire, champs ERP), contour de découpe autour du code, niveau de redondance | **Non** → C5 |
| Épreuve **TIFF** exportée après RIP (PrintProof), « épreuve seule » | Partiel — BAT PDF (`bat.rs`) → C12 |
| Composition multi-copies (Compose) | Oui — planches (1.4 « Fait ») |
| Tex&Repeat (motif répété, textile) | Oui — step & repeat « Fait » |
| Multicouche (blanc, vernis, primaire, ordre des couches, pilote -ML dédié S80600) | Partiel — 2.5 (WH/MS) ; blanc sous/sur fait → C9 |

### 2.3 Imbrication

| Fonction Caldera | Roadmap |
|---|---|
| Imbrication rectangulaire et **vraie forme** (ContourNesting, annoncé −20 % de support) | Oui — `forme.rs` (Fait), 5.1 |
| Action « Nest » : les travaux de **mêmes réglages** se regroupent, sinon plusieurs « Autonest » | Oui — 5.3 |
| Paramètres : rotation auto, réordonnancement, rangées indépendantes, centrage | Oui — 5.1, 5.7 |
| Chargement **Libre** (format fixé par la 1re imbrication) / **Strict** (format = contrainte, séparation par format) | **Non** → C3 |
| Déclencheurs : % de remplissage, **aucun travail reçu depuis X min**, longueur mini, **longueur maxi**, **rangée pleine imprimée à part** | Partiel — 5.2 (manuel, temporisé, surface) → C3 |
| Rouvrir et voir le contenu d'une imbrication (V15) | Oui — 5.4 |
| Massicots (Trim-O-Matik : Fotoba, Crest, Kala ; Fotoba ASP) | Oui — 5.8, 9.9 |

### 2.4 Mosaïque / lés (Tiling+)

| Fonction Caldera | Roadmap |
|---|---|
| Découpage en tuiles, modèles enregistrés | Oui — 3.C1, 3.C2 |
| **Recouvrement / espace / zone blanche différents par tuile** (édition sur les tuiles sélectionnées) | Partiel — 3.C1 (lés inégaux), 3.C3 (soudure) → C13 |
| **Ordre d'impression des tuiles** (TileOrder) | **Non** → C13 |
| Taille certifiée des tuiles (via épreuve TIFF) | Partiel — 3.A1 (règles), C12 |

### 2.5 Couleur et profils (EasyMedia)

| Fonction Caldera | Roadmap |
|---|---|
| Assistant : transitions (encres claires) → linéarisation → limite d'encre → caractérisation ICC (moteur X-Rite i1Prism) → vérification | Oui — 2.2, 8.1, 8.2, 8.3, 8.4 |
| Préréglages de séparation des encres claires (Équilibré / Clair / Élevé) (19.2) | Partiel — 2.2 (Start/Max/End par encre) → C10 |
| Pmax fixé au **pic de chroma** en mesure spectrale (19.2) | Oui — 8.1 (crochet de chroma) |
| **Limite d'encre conseillée calculée** après linéarisation spectrale (19.2) | **Non** → C10 |
| Relinéarisation, cibles « exponentielles » (plus de plages dans les hautes lumières), linéarisation haute dynamique | Oui — 8.2, 8.5 ; cibles exponentielles : Partiel (C10) |
| Paramètres machine portés par le profil (chauffage, aspiration, avance), case par case « valeur du profil / de l'imprimante » | Partiel — 1.1 (le mode porte chauffage/vitesse) ; pas la priorité case par case |
| Synchronisation des profils de la machine (Epson EED → EMX ; HP → .oms) | Oui — 2.9 (lecture EMX) |
| Tons directs : bibliothèques officielles Pantone/RAL/HKS/NCS/PPG, bibliothèque Lab maison, **import d'une bibliothèque fournisseur** (HEXIS Suptac) | Partiel — 8.6 → C11 |
| Préserver le noir pur ; respecter les profils incorporés | Oui — matrice 1.3 « ignorer les profils du PDF » ; noir pur : Partiel (8.3 « noir hors limite ») |
| Alpha → ton direct (blanc, vernis) | **Non** → C8 |
| 2xCMYK, CMYKOGR + séparation post-ICC | Oui — 2.2 (Or/Rd) |
| Encres personnalisées (Custom Inkset) | Oui — 1.1 (groupe = configuration d'encres) |
| **InkPerformer** : économie d'encre (GCR dynamique sur profils existants, 4 niveaux, réglage du départ du noir) | **Non** → C17 |
| **Print Standard Verifier** : conformité Fogra, G7, ISO 12647-2/-7:2016/-8:2021 en ΔE00 (vérification, pas certification) | Partiel — 8.9, 8.10 |
| Formule ΔE au choix (76 / 2000), export automatique des ICC créés vers un dossier | Partiel — 8.9 (ΔE2000) |
| Profil CMJN d'entrée par défaut Idealliance PrintWide2020 (19.2) | **Non** (choix de défaut, P3, rangé dans C11) |

### 2.6 Découpe / print & cut

| Fonction Caldera | Roadmap |
|---|---|
| CutServer (rouleau) / GrandCutServer (à plat) : serveur de découpe, plusieurs contours par fichier | Oui — 4.2, 4.3 |
| Noms de découpe : `CutContour` par défaut + liste éditable, **joker** (`Die*`) | Oui — 4.1 |
| Contour ignoré s'il est sur un **calque non imprimable** (OCG print=off), avec option d'ancien comportement | **Non** → C2 |
| Contrôles du fichier : ton direct CMJN, trait 0,1 pt sans effet (sinon coupe double), trait centré, hors plan de travail | Partiel — 4.5 (RVB, vectorisé coupé 2 fois, boîte) ; centrage du trait et hors plan de travail absents → C2 |
| **Erreur avant impression** si la découpeuse affectée n'est pas utilisable (licence) | **Non** → C1 |
| Repères avancés, repères intermédiaires, marques sur fond (forme, couleur), **repères blancs sur support foncé / transparent** | Partiel — 4.15 (fond sous repères) → C9 |
| Code-barres 39 ou **QR** porteur de l'id de coupe | Partiel — Kona POSTNET fait (`reperage.rs`) → C5 |
| Miroir print & cut (rétroéclairé, sublimation) | Oui — 3.B3 (à éprouver) |
| **Smart Materials** : association auto support d'impression → matière de découpeuse par motif (nom, GUID, type) | **Non** → C7 |
| Fichiers « cut-ready » pour Zünd, Kongsberg (JDF), Summa, DXF générique, ZCC | Partiel — 4.13, 9.9 |
| Réglages de coupe attachés au préréglage, pas à la découpeuse (19.0.2) | Oui — 4.2 (réglage par nom) |

### 2.7 Pilotes

| Fonction Caldera | Roadmap |
|---|---|
| > 1 600 machines annoncées ; **Epson S80600 : pilote standard + pilote -ML (V19)**, 11 encres (C M Y K Lc Lm Lk Or Rd WH MS), USB / fichier / réseau / série, Custom Inkset | Oui — phase 2 (un seul pilote, le nôtre) |
| Détection des machines sur le réseau ; EasySelect (support et formats chargés lus sur la machine) | Oui — 1.3 (SNMP Epson) |
| Statut machine dans l'API (support, encres, bac d'encre usagée, chauffages) | Partiel — SNMP lu (`media.rs`), non publié vers l'ERP → C18 |
| « Transfer at once » : fichier généré en local puis copié d'un bloc vers la destination (SMB) | Partiel — 4.6 (écriture atomique en dossier) |

### 2.8 Rapports et coûts

| Fonction Caldera | Roadmap |
|---|---|
| **CostProof** : encre et support par travail, **calculés à partir des données tramées** (nombre de points × volume de goutte en pL, petite/moyenne/grosse), coût au litre depuis prix/contenance du flacon, support au m² depuis largeur/longueur/prix ; surface imprimée **ou** surface du rouleau | Partiel — `couverture.rs` (estimé sur l'aperçu), 6.1 (SNMP réel) ; **comptage de points absent** → C0 |
| Pas de calcul d'encre en contone (sauf cas HP particuliers) | Oui — 6.7 (« non calculable ») |
| CostView / CostEstimation retirés en V16.3 (seul CostProof reste) ; export CSV non documenté | Oui — 6.5 (nous faisons mieux : CSV) |

### 2.9 Automatisation

| Fonction Caldera | Roadmap |
|---|---|
| Workflows + QuickPrint + HotFolders + Nest-O-Matik combinés | Oui — réglages + dossiers + phase 5 |
| SmartImport (nom de fichier) | Partiel — 9.1 → C4 |
| API REST soumission/suivi, identifiants stables | Oui — 7.1 à 7.4 (et notre webhook rejouable va plus loin que leur interrogation) |
| Suppression auto des travaux API après exécution | Partiel — 0.6 (purge) ; politique par source absente (intégré à C14) |

---

## 3. Ce que Caldera fait et que la roadmap ne couvre pas — items proposés

| ID | Pri | Item | Critère de « fini » |
|---|---|---|---|
| C0 | **P1** | **Encre estimée par comptage de points** (après 2.3) : ml par canal = Σ points de chaque taille × volume de goutte (pL) du mode ; volumes éditables par mode (défauts saisis par nous) ; remplace l'estimation sur l'aperçu dès que le mode est tramé ; reste « non calculable » en contone | test : bande synthétique de N points de chaque taille → ml attendu au µl près ; travail réel : écart estimé/SNMP affiché à côté (6.5) |
| C1 | **P1** | **Pré-contrôle de la découpe avant l'impression** : si un travail porte une découpe et que la découpeuse affectée est absente du catalogue, hors ligne, sans connexion valide ou sans réglage pour une couche, la sortie impression est bloquée avec la raison `decoupeuse_indisponible` (jamais d'encre gâchée) | test file : découpeuse supprimée → impression refusée avant envoi ; découpeuse rétablie → part |
| C2 | **P1** | Compléter 4.5 : **tracé de découpe sur calque non imprimable** (OCG print=off) ignoré par défaut avec alerte, option pour le prendre ; alertes « trait non centré » (intérieur/extérieur) et « tracé hors du plan de travail » | 3 PDF de test → 3 alertes ; option activée → le tracé du calque masqué est coupé |
| C3 | P2 | Compléter 5.2 : déclencheurs **inactivité** (aucun travail reçu depuis X min, distinct du minuteur depuis le 1er travail), **longueur maxi**, **rangée pleine sortie seule** ; chargement **libre** (format figé par la 1re planche) ou **strict** (une planche par format) | test : 3 travaux, inactivité 1 min → départ 1 min après le dernier ; longueur maxi 2 m franchie → départ ; strict + 2 laizes → 2 planches |
| C4 | P2 | Étendre 9.1 : **motif de nom de fichier à balises** (copies, machine, réglage, support, laize, échelle, largeur, hauteur, répétition) qui pré-remplit le travail ; double séparateur conseillé ; journal d'extraction en cas d'échec | test : `Affiche__epson__bache__30.pdf` → 30 copies, réglage Bâche ; nom non conforme → travail importé avec alerte, pas d'échec silencieux |
| C5 | P2 | **QR code / code-barres en marge** avec balises (nom, page, tuile/total, date, id travail, id coupe, machine, réf. commande ERP 5.6), taille, marge, niveau de correction, **contour de découpe autour du code** en option | test : QR du PDF produit décodé = chaîne attendue ; option contour → couche de découpe ajoutée |
| C6 | P2 | **Rendu parallèle** des arrivées (dossiers et API) sur N travaux simultanés, **ordre de sortie = ordre d'arrivée**, import manuel prioritaire sur l'automatique | test : 20 fichiers déposés, 4 rendus parallèles → file dans l'ordre de dépôt ; un import manuel passe devant |
| C7 | P3 | **Association support → matière de découpe** par motif (nom, id, type) pour les découpeuses qui ont des matières (Zünd, CAMM-GL avec presets) ; non associé → défaut + alerte | test : « HEXIS V3000 Vinyle » → matière Vinyle ; support non reconnu → défaut signalé |
| C8 | P2 | Étendre 9.2 : **canal alpha (PNG/TIFF) → plaque blanc ou vernis** au choix du réglage | test : PNG détouré → plaque blanc = masque alpha ± 1 px |
| C9 | P2 | **Multicouche générique** au-delà de blanc sous/sur : N couches ordonnées (ex. blanc-couleur-blanc pour vitrophanie, primaire), et **repères blancs / fond sous repères sur transparent ou support foncé** (complète 2.5 et 4.15) | test : travail 3 couches → 3 plaques dans l'ordre déclaré ; repères sur fond blanc présents sur transparent |
| C10 | P2 | Compléter 2.2 / 8.3 : **préréglages de séparation des claires** (3 profils de bascule nommés par nous), **limite d'encre totale proposée** à partir de la linéarisation mesurée (validée à la main), cibles de linéarisation **plus denses en hautes lumières** | test : préréglage appliqué → courbes attendues ; mesure simulée → limite proposée reproductible ; mire générée avec plus de plages < 20 % |
| C11 | P2 | Compléter 8.6 : **import d'une bibliothèque Lab de fournisseur** au format ouvert (CxF, CGATS ou CSV Lab) — ex. nuancier HEXIS ; jamais de bibliothèque Pantone sans licence ; profil d'entrée CMJN par défaut choisi et documenté | test : import CSV de 50 teintes → 50 tons directs ; doublon de nom refusé ou renommé |
| C12 | P3 | **Épreuve TIFF** après RIP : colorimétrique (ICC) ou valeurs machine, tons directs conservés en canaux / aplatis / supprimés, option « épreuve seule », plafond de taille ; sert aussi à **certifier les cotes** d'un lé | test : TIFF produit = dimensions physiques ± 0,5 mm ; canal blanc présent en mode « conserver » |
| C13 | P3 | Compléter 3.C : **recouvrement et espace par tuile** (pas seulement global) et **ordre d'impression des tuiles** choisi | test : tuile 2 avec recouvrement 20 mm, les autres 10 → plaques attendues ; ordre 3-1-2 → file dans cet ordre |
| C14 | P3 | Compléter 0.6 : **deux seuils** (avertir, refuser), envoi automatique refusé = travail en erreur avec motif ; purge propre aux travaux venus de l'API (ne pas conserver après impression, option) | test : seuil bas → avertissement, seuil critique → refus ; travail API purgé à l'impression si option active |
| C15 | P3 | **Client connecté à plusieurs serveurs RIP** (atelier multi-sites ou 2e serveur), file unifiée dans le client | test : client Tauri avec 2 serveurs de test → les deux files visibles, envoi vers chacun |
| C16 | P3 | **Rééchantillonnage bicubique** (ou meilleur) à l'agrandissement des images matricielles, choix par réglage | test : image 100 ppi ×4 → PSNR meilleur que plus proche voisin sur une mire |
| C17 | P3 | **Économie d'encre** par GCR (remplacement du gris par le noir) appliqué à un profil existant, niveaux réglables, départ du noir réglable — via le moteur libre (ArgyllCMS), jamais un module tiers | 🏭 ΔE2000 moyen ≤ 2 entre profil d'origine et profil économe sur la mire, encre totale réduite et mesurée |
| C18 | P2 | Compléter 7.2/7.4 : **état machine publié à l'ERP** (support chargé, niveaux d'encre, **bac d'encre usagée**, chauffages, alertes) par événement et à la demande | test : relevé SNMP enregistré → événement `machine.etat` avec les mêmes valeurs |

Décompte : **P1 = 3** (C0, C1, C2) · **P2 = 9** (C3, C4, C5, C6, C8, C9, C10, C11, C18) · **P3 = 7** (C7, C12,
C13, C14, C15, C16, C17) · total **19** items.

Ce que la roadmap fait **mieux** ou autrement que Caldera (à garder) : webhook signé à séquence rejouable (Caldera :
interrogation REST) ; consommation **réelle** SNMP par travail (CostProof reste un calcul) ; CSV de rapports (non
documenté chez Caldera) ; reprise sur fichier d'état illisible ; historique des linéarisations.

---

## 4. Format des profils/média Caldera et profils HEXIS « Caldera »

### 4.1 Format

| Fait | Source |
|---|---|
| Un profil média Caldera s'échange en **paquet propriétaire `.calpatch`** (« Media Profile Patch »). Il regroupe la **courbe de transition** des encres claires (si besoin), la **courbe de linéarisation** et la **table ICC**. Export en un ou plusieurs paquets (« One Big Patch »), import par « Install Patch ». | helpdesk 360005357477 |
| Un mot de passe par défaut est parfois demandé à l'import (selon Avery Dennison). | bulletin Avery TB 5.13 |
| Le profil porte aussi des **paramètres machine** (chauffage, aspiration, avance) et un commentaire de 170 caractères ; coût au m². | EasyMedia overview ; guide HEXIS |
| EasyMedia peut **exporter automatiquement les ICC qu'il crée** dans un dossier (pour l'épreuvage en prépresse). | EasyMedia overview |
| Profils fournis par la machine : Epson (EMX via Epson Edge Dashboard, « AutoSync »), HP Latex (`.oms`, format HP qui contient réglages + ICC). | helpdesk 4403967199121 ; recherche « .oms » |
| Bibliothèque de profils Caldera : via CalderaDock depuis V12 (paquets). | EasyMedia overview |

### 4.2 Profils HEXIS pour Caldera

- Le guide HEXIS (EFU.DOC.14A) indique que le fichier téléchargé pour Caldera est un **`.calpatch` ou un `.oms`**
  (HP), éventuellement compressé. **Ce n'est pas un `.icc` standard** : c'est un paquet propre au RIP.
- Ce paquet contient, d'après HEXIS : paramètres machine (chauffage, aspiration…), **calibrage des quantités d'encre**
  (linéarisation/limites) et ICC. HEXIS précise lui-même qu'un profil ne vaut que pour **un couple RIP – imprimante –
  encre – mode**, et que ses profils sont « génériques » (faits sur une autre machine) : à refaire sur la machine ou au
  minimum à relinéariser.

### 4.3 Conséquences pour E-KO RIP Center (règle rappelée)

1. **Ne pas utiliser** les profils HEXIS « Caldera » : livrés en `.calpatch`, en sortir l'ICC serait une **extraction
   d'un paquet propriétaire** → interdit par notre règle.
2. Même un ICC obtenu par voie légitime chez un utilisateur Caldera (export EasyMedia) décrit la **séparation, les
   transitions et la linéarisation de Caldera** : ses valeurs machine n'ont aucun sens pour notre séparation 9 encres
   (2.2) et notre tramage (2.3). Usage au plus **provisoire** : épreuvage écran, gamut indicatif, point de départ.
3. Ce que l'on peut reprendre **légitimement** du guide HEXIS : des **faits publiés** (températures de chauffage
   recommandées par support, conseils résolution/passes, recommandation sRGB + Coated FOGRA39 + perceptif, mesure des
   tons directs en ΔE avec seuils 1 / 2,5) → données de nos supports (1.1, 1.5), jamais un fichier.
4. Piste pour 1.5 : dans la base HEXIS, choisir une combinaison qui livre un **`.icc` nu** (ex. RIP « autre » ou ICC
   générique) si elle existe — **non vérifié** (la recherche HEXIS passe par un formulaire, non soumis). À faire par
   Mathieu au navigateur.
5. Profils de production : les nôtres, i1Pro + ArgyllCMS, phase 8 (inchangé).

---

## 5. Sources lues

Caldera (site) :
- https://www.caldera.com/calderarip-version-comparison/
- https://www.caldera.com/why-choose-caldera/
- https://www.caldera.com/solutions/version-19/
- https://www.caldera.com/solutions/calderaconnect-rest-api/
- https://www.caldera.com/solutions/calderarip-subscription/
- https://www.caldera.com/caldera-releases-new-calderarip-update/ (V18.1)

Caldera HelpDesk :
- https://helpdesk.caldera.com/hc/en-us/articles/38260212577041-CalderaRIP-V19-Changelog
- https://helpdesk.caldera.com/hc/en-us/articles/27821478288785-CalderaRIP-V18-0-Changelog
- https://helpdesk.caldera.com/hc/en-us/articles/30164546973969-Caldera-REST-API-V2-Changelog
- https://helpdesk.caldera.com/hc/en-us/articles/21882880683793-Client-Server-Architecture-in-CalderaRIP-Overview
- https://helpdesk.caldera.com/hc/en-us/articles/4407521893393-Nexio
- https://helpdesk.caldera.com/hc/en-us/articles/4408132770065-HotFolders-Smart-HotFolders
- https://helpdesk.caldera.com/hc/en-us/articles/4405877114513-SmartImport-Overview
- https://helpdesk.caldera.com/hc/en-us/articles/4403967184401-How-to-use-and-configure-Nest-O-Matik
- https://helpdesk.caldera.com/hc/en-us/articles/4407869992593-Spooler-Queues
- https://helpdesk.caldera.com/hc/en-us/articles/360019238954-CostProof-Overview
- https://helpdesk.caldera.com/hc/en-us/articles/4406062467857-EasyMedia-Overview
- https://helpdesk.caldera.com/hc/en-us/articles/360005357477-How-to-Export-Import-ICC-Profiles-using-EasyMedia
- https://helpdesk.caldera.com/hc/en-us/articles/4403967199121-How-to-import-EMX-files-from-EED-to-EasyMedia
- https://helpdesk.caldera.com/hc/en-us/articles/360019821593-CutContour-Troubleshooting-Extended-Version
- https://helpdesk.caldera.com/hc/en-us/articles/29386641565841-Cutter-Tools-Configuration-Smart-materials
- https://helpdesk.caldera.com/hc/en-us/articles/4403971784977-How-to-use-the-QR-codes-from-the-Marks-tab
- https://helpdesk.caldera.com/hc/en-us/articles/4408289065105-PrintProof-Overview
- https://helpdesk.caldera.com/hc/en-us/articles/4403971816209-Epson-SureColor-S80600-L-Overview
- https://helpdesk.caldera.com/hc/en-us/sections/4403966835345-Marks-tab (sommaire)
- pages de résultats de recherche du HelpDesk (titres et extraits seulement)

Tiers :
- https://hexis-graphics.com/documents/guide/document_en/EFU.DOC.14A-EN_Configuring_Caldera_to_print_on_your_HEXIS_media.pdf (guide HEXIS, PDF de documentation)
- https://www.hexis-graphics.com/fr/supports/impression-numerique/profils-icc/ (conditions d'usage ; formulaire non soumis)
- Extraits de résultats de recherche (non ouverts) : bulletin Avery Dennison TB 5.13 ; articles de presse 2009
  (APPE, prix DPI SGIA 09), datasheets CutServer / InkPerformer, Enfocus Switch × Nexio, wiki PrimeCenter.

Points incertains (non revérifiés à la source) : inclusion de Nexio dans les abonnements depuis nov. 2025 ; prix et
nombre réel de pilotes additionnels par palier ; nature de l'« accélération matérielle » (GPU ?) ; existence d'un `.icc`
nu chez HEXIS pour une combinaison donnée ; détail des routes REST au-delà des trois citées (OpenAPI non lue).
