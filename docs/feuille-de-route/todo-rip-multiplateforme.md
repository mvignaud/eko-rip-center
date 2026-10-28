# RIP multiplateforme (clone fonctionnel d'ONYX Thrive 25.5) — 2026-10-23

Usage interne uniquement. Code 100 % neuf. Cadre fixé avec Mathieu :
- autorisé : inventaire des fichiers, configuration et documentation de l'installateur ;
  sources publiques (fabricants, normes) ; observation de ce que le RIP envoie aux machines.
- interdit : décompiler les binaires ONYX, reprendre leurs profils / pilotes / tables
  couleur, toucher à la clé HASP.
- risque signalé (non bloquant, décision de Mathieu) : EULA clause (a) et clause (g)
  « using the Software to develop a competitive product ».


## TABLEAU DE BORD — 24/10/2026 (soir)

Publié : v0.1.0 → v0.13.1 (v0.12.0 jamais publiée). Serveur de l'atelier (192.168.10.109) en
0.13.0 le 24/10 ; sudo temporaire SUPPRIMÉ le 24/10 (mises à jour par Mathieu, mot de passe).
ROADMAP COMPLÈTE : tasks/roadmap-rip-onyx.md (101 items, 10 phases ; 5 premiers : 0.1, 0.3, 1.1,
2.1, 3.A1 ; 5 questions au gérant). Pont ERP : ordre R1→R5 VALIDÉ par Mathieu le 24/10.
Page publique app.e-ko.fr/changelog-rip : branche feat/changelog-rip (eko-source, worktree),
FR/EN, abonnement séparé du changelog ERP — rien déployé.
Contenu des versions 0.11 → 0.13 :
imbrication sur la forme, préparation du pilote Epson (canaux, couches, découpe jamais
envoyée), cadres Kona Types 2/3 + POSTNET, rôles admin/opérateur, sauvegarde de la
configuration, décalage de découpe, step & repeat demi-saut, durée estimée, notifications,
doc d'administration + FAQ, CI d'installation complète de l'ISO.

Signature des clients (Apple Developer ID + notarisation 99 $/an ; Windows Trusted Signing ≈ 10 $/mois) : REPORTÉE par Mathieu le 24/10, « on attend d'avoir des softs suffisamment avancés ».

Reste SANS matériel : règles de dossiers surveillés (R), glossaire, notification système
dans le client Tauri à vérifier, P5 EMX / profil Epson Wide Gamut.

Reste À L'ATELIER (Mathieu) : pilote Epson (capture faite : ESC/P2 tramé 2 bits, 9 canaux — séparation + tramage à écrire), Kona : Type 3 (MF) à éprouver, couches V3000-Blanc LAISSÉE VIDE (décision de Mathieu le 24/10), Graphtec HP-GL/ARMS, Roland (découpe + capture
impression), cadences réelles, décalage de découpe mesuré, linéarisation / limites d'encre.

## PONT ERP ↔ RIP (intégration façon ONYX Align) — lots R1→R5 VALIDÉS par Mathieu le 24/10

Sources : demandes de l'ERP (session EKO SaaS, 24/10 ; détail champ par champ dans
`eko-source/tasks/onyx-align/feuille-de-route.md` § 6.3 et § 6.5), roadmap RIP phase 7
(`tasks/roadmap-rip-onyx.md`). Décisions du gérant : D7 fichiers par lien signé (aucun dépôt
local), D8 AUCUN montant vers l'ERP, D9 pas d'envoi auto au RIP le 1er mois du pilote, D10
avancement = opérateurs + RIP. Le RIP SORT vers l'ERP (HTTPS) ; aucun port ouvert à l'atelier.
Côté ERP, le chantier démarre après la série comptable (sauf lot A0).

### R1 — Fondations
- [x] R1.1 (v0.16.0) `uuid` immuable sur machines, préréglages, profils, couches de découpe (généré à la
  création, renvoyé par le client, refus si un uuid ayant de l'historique disparaît) ; l'id texte
  et le nom restent libres. Test : renommer `epson` garde son uuid ; supprimer une machine avec
  sorties → refus codé. (demande ERP n°1)
- [x] R1.2 (v0.16.0) Jeton d'API propre à l'ERP (portée : routes du pont seulement, haché au repos,
  rotation) + secret HMAC partagé pour signer les événements sortants. Un serveur « sans compte »
  reste refusé aux appels machine. Test : jeton valide → 200 sur le pont, 403 ailleurs. (n°2, 7.1)
- [x] R1.3 (v0.16.0, docs/fr/pont-erp.md) Modèle d'états PUBLIÉ (doc FR/EN) : états d'un travail et de ses sorties, table de
  correspondance avec les états de l'ERP, état après coupure (sortie en cours → `echec`
  `interrompu`, travail → reprise). (n°6)
- [x] R1.4 Aucun montant vers l'ERP : `/api/tarifs` reste local (acquis). (n°11, D8)

### R2 — Canal
- [x] R2.1 (25/10, pont.rs) Catalogue unique (machines, supports, modes, préréglages, découpeuses, par uuid)
  + événement `catalogue.modifie` à chaque changement. Supports et modes : dépend de la phase 1
  (1.1) de la roadmap. (n°4, 7.2)
- [ ] R2.0 (25/10) Contrat côté ERP prêt chez EKO SaaS : `eko-source/docs/onyx/pont-erp-cote-erp.md`
  (branche feat/onyx-pt1-catalogue-rip, lot PT1) — base `https://app.e-ko.fr/api/v1/gestion-imprimerie/rip/<slug>`
  (pilote : goldenprint), `POST /evenements` (dernier_seq_traite, 409 seq_manquant), `GET /en-attente`,
  `catalogue.modifie` (remplacement par genre, uuid obligatoire). Proposé par le RIP : champ `instance` (UUID
  de l'instance du pont, dans erp.json + sauvegarde) dans l'enveloppe, clé ERP (instance, seq) ; anomalie
  dernier_seq_traite > seq même instance → arrêt + alerte Système. ADOPTÉ par EKO SaaS le 25/10 (inclus dans PT1 avant fusion ; événement sans instance = instance « par défaut » côté ERP).
  Règles convenues pour l'implémentation R2 (25/10) : `GET /en-attente?instance=<uuid>` toujours envoyé ;
  réponse 0 + instance JAMAIS accusée → rejouer depuis seq 1 ; instance DÉJÀ accusée → « remplacé » :
  plus rien envoyé ni pris, alerte Système, relance seulement par l'admin (nouvelle instance) ;
  Garde ERP `instance_concurrente` : messages croisés le 25/10 (ajoutée puis retirée) — dernière annonce
  d'EKO SaaS : RETIRÉE, règle simple (« remplacé = se tait ») seule. SOURCE DE VÉRITÉ = eko-source
  docs/onyx/pont-erp-cote-erp.md, à RELIRE au démarrage de R2 (si le 409 instance_concurrente y figure :
  ni travail ni envoi, alerte Système, nouvel essai /5 min).
- [x] R2.2 (25/10, pont.rs) Canal sortant : le RIP tire les travaux/ordres en attente (HTTPS) et pousse ses
  événements numérotés, rejouables « depuis N » (journal local, renvoi après coupure réseau).
  Test : ERP injoignable 10 min → aucun événement perdu, ordre conservé. (n°3, 7.4)
- [x] R2.3 (25/10, pont.rs) Ressources réellement utilisées (machine, support, mode) dans l'événement de fin. (n°8)

- [x] (25/10, verrou de test `DECOUPE_GLOBALE`) Test flaky : `sauvegarde_et_restauration_de_la_configuration` réinstalle les réglages de découpe GLOBAUX (decoupe::installer) pendant que d'autres tests tournent — isoler le global par test.

### R3 — Réception des travaux
- [x] (28/10, envoi = version ; renvoi = nouvel envoi) R3.1 Réception par référence ERP + version, fichier par lien signé, SHA-256 vérifié,
  idempotente (même réf + version = même travail) ; refus codé stable d'une combinaison
  inexistante (machine × préréglage d'abord, machine × support × mode quand 1.1 existe). (n°5, 7.3)
- [ ] R3.2 Actions sur la file par référence ERP : suspendre, reprendre, annuler, priorité ;
  un travail imprimé ne s'annule pas, il se signale. (n°10, 7.7)

### R4 — Estimation à la demande
- [ ] R4.1 Depuis `/api/planches/apercu` (sans création) : longueur, lés, surface, durée
  (`m2_heure`), encre estimée par canal ; mêmes valeurs que la fiche une fois le travail créé. (n°9, 7.6)

### R5 — Consommation réelle par travail (dépend du pilote Epson, phase 2)
- [ ] R5.1 Attribution par fenêtre début/fin de sortie (`Sortie.debut/fin` + `releves::consommation`) :
  support sur toute la laize (mm), gâche, temps machine ; encre en ml par canal = estimation ×
  étalonnage (le % SNMP est trop grossier : afficher « < seuil » sous 1 %). (n°7, 7.5)
- [ ] R5.2 États « en impression » / « imprimé » réels, émis par le pilote direct. (n°12)

## Inventaire de l'installateur (fait le 23/10)

Installateur : SFX RAR5 signé, 4,23 Go, sha256 2391b420…993a4b. 762 fichiers, dont
`Install/Setup/Setup.zip` (1,3 Go, 4 671 fichiers) = l'application.

| Module ONYX | Rôle | Équivalent prévu |
|---|---|---|
| PDFRip (Adobe APPE, ACRFonts/PDFL), XRip, PSRip | rendu PDF/PS | Ghostscript (séparations, surimpression, tons directs) |
| Server, PosterShop, PrintDirect, JobArchiver | files d'attente, impression, archivage | serveur de travaux |
| Dashboard (web), oatpp + swagger | interface web, API REST | interface web + API |
| MediaManager, Profiles, Tools/ProfileOptimizer, CDevs (EyeOne, SpectroProofer, CGATS…) | supports, limites d'encre, linéarisation, création ICC, spectros | ArgyllCMS + LittleCMS |
| PrinterRepository : 325 paquets | pilotes d'imprimantes | Gutenprint pour Epson/Canon, protocoles publiés (RTL, PCL3GUI…), observation |
| CutServer + db/Cutters (≈ 100 plotters) + Common/*Mark.dll | découpe, repères print & cut | module découpe (HP-GL, MH-GL, GP-GL, DM/PL…) |
| Preflight, MediaAnalyzer | contrôle avant impression | qpdf / Ghostscript |
| OnyxDB (MongoDB) | base | SQLite |
| LicenseManager, hasp_rt | licence | aucun |

Mutoh KONA est géré par ONYX : `db/Cutters/Mutoh KONA` (cutterSupport 401, markType 13,
module de repères `mutohkonamark`) et `CutServer/lib/mutohusb.dll` pour l'USB.

## Décisions (23/10)

- Rust (moteur + serveur) et interface web. Un serveur local sous **Linux** (choix
  corrigé par Mathieu le 23/10, Windows écarté), auquel sont branchées les machines ;
  les postes clients (Mac, Linux, Windows) passent par le navigateur.
- Kona sous Linux : module noyau `ftdi_sio` + `new_id` `0c30 6010` (règle udev) →
  `/dev/ttyUSB0`, aucun pilote à écrire. À éprouver machine branchée.
- Dépôt : https://github.com/mvignaud/eko-rip-center
- Parc actuel : **Epson SureColor SC-S80600** (encres blanc et argent métallisé en
  couleurs spéciales) et **Mutoh Kona 1650**.
- Nom du logiciel : **E-KO RIP Center** (dossier et crate : `eko-rip/`).
- Nuanciers fournis (Dropbox `05 - RESSOURCES/GRAPHISME/NUANCIERS ILLUSTRATOR/`) :
  - `Nuancier Epson Metallic Color SC-S80600 - Illustrator CC.tab` : 289 couleurs
    `MT-*` en Lab cible (virgule décimale). Couleurs métallisées composées (argent +
    CMJN) : le fichier donne la cible, PAS la recette → recettes à mesurer (P5).
  - `Printcom_Swatches_01-2022.ase` : tons directs techniques. `White` ; découpe
    `Kisscut`, `Thrucut`, `Insidecut`, `Vcut`, `Perforation`, `Creasing`, `Score` ;
    autres `Drilling`, `Numbering`, `Scodix *`, `Cold Foil` → routage en P3.

## P0 — détail

- [x] serveur axum sur le réseau local (`EKO_RIP_ADDR`, `EKO_RIP_DATA`, `EKO_RIP_GS`)
- [x] envoi d'un PDF, contrôle de l'en-tête et de la taille, identifiant de travail généré
- [x] rendu Ghostscript `tiffsep` en `-dSAFER` : CMJN + une plaque par ton direct
  (blanc, argent) ; aperçu basse définition en PNG
- [x] un travail = un dossier + `job.json` (pas de base en P0)
- [x] page web : envoi, liste des travaux, aperçu et plaques
- [x] test : PDF avec tons directs `White` et `Silver` → plaques attendues présentes
- [x] preuve : `cargo test` vert + parcours réel dans le navigateur

Livré le 23/10 : commit da949e0 sur origin/main (mvignaud/eko-rip-center, privé).
Preuve : `cargo test` vert (plaques White/Silver, polarité, pas de fuite du blanc
dans le noir) ; clippy propre ; API éprouvée (3 refus en 400, chemin `../` refusé,
id invalide 400) ; envoi réel par le formulaire dans le navigateur, 6 plaques affichées.

## Livraison (consigne de Mathieu, 23/10)

« Créer les release et package au fur et à mesure dès que tu as des choses
intéressantes » ; « 1 package serveur + 1 client avec changelog, documentations,
traduction (FR/EN pour l'instant) ».

- [x] espace de travail Cargo : `serveur/` (binaire actuel) + `client/` (Tauri)
- [x] client : demande l'adresse du serveur au premier lancement, la retient,
  ouvre l'interface du serveur dans sa fenêtre (dossier surveillé, imprimante
  virtuelle : plus tard)
- [x] interface FR/EN (langue du navigateur, bascule manuelle) ; erreurs de l'API en
  codes stables traduits côté écran
- [x] `CHANGELOG.md` + `docs/fr/` et `docs/en/` (installation serveur, installation
  client, utilisation)
- [x] workflow sur tag `v*` : image `ghcr.io/mvignaud/eko-rip-center` + binaires
  serveur Linux + paquets client (dmg, msi, deb/AppImage) attachés à la release
- [x] `v0.1.0` publiée le 23/10 : 8 fichiers (2 serveur, 6 client), image ghcr 0.1.0 + latest
  (sha256:bd8952f2…), .deb inspecté. NON éprouvé : installation réelle du .deb sur
  Debian, `docker run` de l'image, lancement des paquets client Windows/Linux
- [x] image Debian amorçable (demande de Mathieu, 23/10) → `v0.2.0` (remplacée par l'ISO Ubuntu 24.04, v0.10.0) :
  - [x] script `scripts/construire-iso.sh` : netinst Debian 13 officiel (SHA256 vérifié)
    + `preseed.cfg` + `.deb` serveur sur l'ISO, menu « Installer E-KO RIP Center
    (EFFACE LE DISQUE) », démarrage BIOS et UEFI
  - [x] préconfiguration : FR, clavier fr, nom `eko-rip`, disque entier, SSH, paquet
    serveur installé ; SEULS les mots de passe sont demandés (aucun secret dans l'ISO)
  - [x] preuve (24/10) : ISO v0.2.0 PUBLIÉE installée en VM UEFI en 20 min ; seules questions =
    mot de passe + écriture disque ; service actif et activé, paquet 0.2.0-1, écran 200, PDF rendu
    (6 plaques), SSH, adresse sur la console (127.0.1.1 au tout premier affichage, puis l'IP)
  - [x] workflow : ISO jointe à la release ; doc FR/EN ; CHANGELOG 0.2.0
- limite connue : paquets client NON signés (Gatekeeper / SmartScreen avertiront)
  tant qu'aucun certificat Apple / Windows n'est fourni

## P2 — découpe Mutoh Kona (détail)

- [x] extraction des tracés de découpe du PDF : lecture du contenu de page (lopdf),
  état graphique (`cm`, `q`/`Q`, formulaires XObject), chemins (`m l c v y h re`),
  couleur de trait/remplissage en Séparation ; tons reconnus : `CutContour`,
  `Thrucut`, `Kisscut`, `Insidecut`, `Perforation`, `Creasing`, `Score`, `Vcut`
  (casse et espaces ignorés)
- [x] aplatissement des courbes de Bézier (tolérance ~0,025 mm), unités du plotter
  (HP-GL : 40 unités/mm), origine et sens des axes réglables
- [x] écriture HP-GL (`IN;` `SP1;` `PU x,y;` `PD …;` `PU;` `SP0;`), un outil par ton
  (lame / pointe à tracer / perforation selon le Kona)
- [x] envoi : fichier .plt téléchargeable depuis l'écran, puis port série
  `/dev/ttyUSB0` (9600 8N1, contrôle de flux matériel — à confirmer sur la machine)
- [x] tests : PDF à carré + cercle en `CutContour` → coordonnées HP-GL attendues ;
  tons non découpe ignorés ; formulaire imbriqué avec `cm`
- [x] preuve en VM (24/10) : ISO 0.2.0 + .deb CI ; puce FTDI émulée (QEMU usb-serial, 0403:6001) →
  /dev/ttyUSB0 groupe dialout, service SupplementaryGroups=dialout, règle udev validée par
  `udevadm verify` ; envoi depuis l'écran → 1260 octets reçus IDENTIQUES au .plt (crtscts actif)
- [ ] preuve sur machine réelle : identifiant 0c30:6010 → /dev/kona ; langage MGL accepté ;
  coupe d'un carré de 100 mm mesuré au réglet

Deux modes demandés par Mathieu (24/10) :
- [x] **découpe seule** : fichier AI, PDF ou SVG ; choix « tons de découpe » ou « tous les
  tracés » ; SVG lu par usvg (transformations, formes, courbes)
- [x] **impression puis découpe avec repérage Mutoh** : le Kona lit un CADRE (bounding box),
  des repères intermédiaires et un code-barres (multi frame). La norme est dans le
  manuel Kona, chap. 6 « Bounding Box Specifications » — PAS dans ONYX (logique dans
  `MutohKONAMark.dll` / `mutoh-4.2.1.10907.jar` : code, on ne décompile pas).
  Sources demandées à Mathieu : PDF/CD du manuel Kona, OU sorties d'ONYX observées
  (travail imprimé avec repères exporté, fichier de découpe CutServer envoyé vers un fichier).
  Public (Wasatch) : USB = port COM ; firmware ≥ 6.0 ; Page Mode 1 ; TP1 en bas à droite ;
  ordre de lecture manuel 1, 3, 4, 2 ; automatique : Contour Cut > Automatic > Multi Frame.
  **Manuels fournis par Mathieu (24/10)** : User's Guide AP-75380 Rev 1.2 et CutServer /
  Print & Cut Application Guide (uploads/…23733749.pdf et …426144.pdf). Retenu :
  - émulation MGL ; pas 0,010 mm OBLIGATOIRE en contour ; vitesse série sans effet (USB)
  - code-barres POSTNET, 20 mm de haut, ≥ 246 mm de large, n° de 11 chiffres, haut + bas
  - Type 1 (sans code-barres ni CutServer) : épaisseur 3–20 mm, cadre ≥ 250 × 250 mm,
    image à ≥ 6 mm du cadre, avant 25–1200 mm, arrière 75 mm (50 entre deux cadres),
    galet ≥ 1 mm du cadre
  - Type 2 (SF) : + code-barres à 40–100 mm du bord horizontal, 6–12 mm vertical,
    image/n° à 2–6 mm du cadre
  - Type 3 (MF) : + repères intermédiaires 10–60 mm, repères début/fin 125 mm de haut
  - CutServer = dossier surveillé ; fichiers `Nom_SF|MF|NC_<11 chiffres>.plt` ; le Kona
    lit le code, CutServer renvoie le fichier du même numéro
  - OBSERVÉ (24/10) : ONYX écrit `C:\ONYXGo25\cutter\Mutoh KONA\<id>_<11 chiffres>.xml`
    (« cut-list » interne RIP → CutServer, en pouces) : 4 `reg-mark type="onyx"` aux coins
    d'un cadre de 302 × 305 mm pour une image de 106 × 112 mm, `cut-path` Cutcontour en
    Bézier, `TileOutline` = rectangle de l'image. Le numéro à 11 chiffres du nom = code-barres.
    Ce format ne sert pas au Kona : reste à capturer CutServer → Kona en USB (USBPcap)
  - **CAPTURE USB CutServer → Kona (24/10)** : COM4 = « USB Serial Port (Kona) »
    FTDIBUS VID_0C30 PID_6010 ; 9600 8N1, RTS/CTS, DTR+RTS levés. Flux (lignes en `;\r\n`) :
    `AL5,0;IN;` · `PA;PROGSTEP10;` · `@SEGMENTEDFILESTART1;` · `@SEGMENTSTART1;` ·
    `AL5,0,0,1,x1,y1,x2,y2;` (coins opposés du cadre, MILIEUX des traits, 0,01 mm) ·
    `AS 1.0;ZF 145;VS 15;LT;` (accélération G, force g, vitesse cm/s = couche CutServer
    « Mi-chair V3000 ») · PU/PD absolus · TileOutline coupé aussi · `PU x1,y1;` · `AL5,2;` ·
    `@SEGMENTEND1;` · `SP0;@SEGMENTEDFILEEND1;`. Le Kona répond `%KN1650 178153;`, `%OK;`
  - **ESSAIS DIRECTS (24/10, PowerShell → COM4)** : sans `SP1;` rien ne bouge (outil 0) ;
    avec `SP1;` le Kona coupe. Depuis le point 0 : X+ = vers le fond de la feuille (média
    sortant vers l'avant), Y+ = vers la GAUCHE ; Y négatif → « hors limites, rognées en zone
    Hardclip ». Repère direct : X_kona = y_page, Y_kona = −x_page (comme ONYX)
  - couches CutServer : Mi-chair V3000 (défaut) 15 cm/s, 145 g, 1 G ; V3000WG - Laminé 15 cm/s,
    305 g, 1 G (dans le catalogue par défaut du Kona) ; V3000-Blanc : valeurs à relever
  - **✅ PRINT & CUT DU RIP VALIDÉ (24/10, 17:50)** : cadre Type 2 + POSTNET du RIP imprimé par
    ONYX (couteau Aucune), envoyé au Kona en `AL5` par COM4 → cadre et codes lus, triangle coupé,
    décalage NUL (« totalement conforme »). Type 1 seul : EPOS 6213 (le Kona exige un code ;
    manuel § 6.2) → utiliser le Type 2. Flèche ▲ : vers le haut de page (bord sorti en premier
    de l'Epson, engagé en premier dans le Kona par l'arrière)
  - SNMP Epson : longueur restante du rouleau = prtInputCurrentLevel (µm), décroît pendant
    l'impression ; travail de 369,8 mm → 574 mm consommés (+204 mm machine) ; alertes
    (« printer cover open », « execute paper initialization »…) ; ST2 = niveaux d'encre
    (10 entrées, à attribuer avec l'écran de l'Epson)
  - ST2 décodé (24/10) : champs [type, longueur, données] après « @BDC ST2\r\n » + longueur 16 bits.
    0x2d = (02, N° de SUPPORT Epson) CONFIRMÉ (07 au travail Lions = « support 7 » d'ONYX, 06
    après changement = « support 6 ») ; 0x0f = encres, entrées de 4 octets (repère, code couleur,
    %, capacité) : 00 K 93, 01 C 23, 02 M 75, 03 Y 76, 04 Lc 52, 05 Lm 41, 08 Lk 54, 09 Rd 79,
    0D Or 10, 10 = CL LIQUIDE DE NETTOYAGE 22 (capacité 50) — CONFIRMÉ sur l'écran de l'Epson
    (OR LK BK C M Y R LC LM CL ; pas de WH/MS chargé aujourd'hui) ; 0x2e = chauffages (consigne,
    mesure) ×3 : 38/38/50 °C, conforme à l'écran ; 0x1f = n° de série X27S003543 ; 0x19 = nom du support
  - [ ] Système > Média : afficher longueur restante, n° de support, niveaux d'encre (SNMP) ;
    avertir si un travail dépasse la longueur restante
  - ~~INCONNU : syntaxe MGL de la « frame scanning command »~~ → AL5 (ci-dessus)
  - (ancien) : syntaxe MGL de la « frame scanning command » placée dans le .plt
    → demandé : un .plt SF et un MF du dossier CutServer produits par ONYX + l'impression
- [x] cadre Types 1/2/3 aux cotes + POSTNET (24/10 : POSTNET 62 barres vérifié en décodant l'exemple du guide ; fenêtres des coins, code du haut, écart des repères MF lus sur schémas) (standard 7-4-2-1-0, chiffre de contrôle,
  barres d'encadrement) ; à confronter au premier .plt / impression réels

## Epson SC-S80600 — documentation publique (24/10)

Page support : https://www.epson.fr/fr_FR/support/sc/epson-surecolor-sc-s80600/s/s1450
(aucun pilote ni protocole ; manuels seulement). Retenu :
- « Printing with WH/MS » (epson602962eu.pdf) : tons directs nommés `White` (WH) et
  `Metallic` (MS) conseillés ; ordre des couches = ordre des calques ou réglage RIP
  (blanc dessous / dessus) ; surimpression décide couleur sur blanc ou réserve
- « Manual for RIP Printing » (epson603028eu.pdf) : fichiers EMX par support (Epson
  Control Dashboard) = réglages imprimante + profil ICC, résolution, passes pour le RIP
- [x] P1 : table tons → encres : WH ← `White`/`Blanc`/`WH` ; MS ← `Metallic`/`Silver`/`MS`/`Argent`
- [x] P1 : ordre des couches WH/MS/couleur réglable par travail (dessous / dessus ; 3 couches couleur-blanc-couleur : à voir)
- [x] P1 : les tons de découpe (`CutContour`…) sortent en plaque : NE JAMAIS les envoyer à
  l'imprimante ; l'aperçu composite les montre dans leur couleur de remplacement (à masquer)
- [ ] P5 : lire les EMX (profil, résolution, passes) pour ne pas refaire les profils
- [x] P5 : profil CMJN d'entrée au choix (dépôt de profils ICC + choix par travail, fait), dont « Epson Wide Gamut » (EpsonWideCMYK_Ver2.icc,
  zip epson602732eu fourni le 24/10 ; © Epson : lu depuis un dossier de profils du serveur,
  JAMAIS embarqué dans le dépôt ni les paquets ; CMJN seulement, pas RVB ni tons directs)
- **mode contone (24/10)** : base ONYX (données, pas code) `db/EPSON SureColor S80600/Defaults/
  mode_Settings_CMYK|CMYKW|CMYKMs.xml` : « CMYKcmkORW/Ms Epson Precision Dot », Pattern = Contone,
  5 canaux 8 bits dans l'ordre K C M Y + W (ou Ms), Res `360#720x1440-CT`. Note Fiery XF 6.3 (publique) :
  pilote « contone » CMYK, imprimante en réseau (IP), supports enregistrés par Epson Control Dashboard
  (EMX), pas de réglage RIP #0. ⇒ le RIP envoie 5 plaques en tons continus ; l'imprimante répartit
  sur ses encres claires/orange/rouge et trame. Résolutions S80600 : 360×720 1 passe (94 m²/h),
  720×720 4 p (28), 720×1440 6 p (17), 1440×1440 16 p (6,6).
- **CAPTURE ONYX → EPSON ANALYSÉE (24/10, travail Stickers_Lions_club_10cm, 720 8p HQ,
  HEXIS V300WG1)** — observation de notre propre machine, outils dans scratchpad/outils :
  - transport : TCP brut **port 9100** ; état par SNMP privé Epson 1.3.6.1.4.1.1248.1.2.2.1.1.1.4
    (« @BDC ST2 », relevé toutes les 10 s) ; réponse 9100 « @BDC PS » (nom du travail)
  - flux : 16 384 octets 0x11, puis `ESC 01 @EJL 1284.4` / `@EJL`, `ESC @`, remote mode
    `ESC (R … REMOTE1` (JS nom, JH poste, JU utilisateur, HD x3, SN, AC, EX, DR), `ESC 00 00 00`
  - réglages : ESC (G 01 · ESC U 00 · ESC (K 00 00 · ESC (i 01 f3 00 (microweave) ·
    ESC (e 00 11 (point variable) · ESC (U 08 04 04 base 2880 · ESC (C / (c / (S (page) ·
    ESC (D 14400/20/20 = **720 × 720**
  - données : **ESC/P2 raster TRAMÉ, 2 bits/pixel (4 tailles de point)**, PackBits, bandes
    de 16 lignes (`ESC (v` +16, `ESC ($` X, `ESC i`, CR) envoyées dans l'ordre de la page :
    l'imprimante fait elle-même les 8 passes ; fin FF, LD, JE, JI
  - **9 canaux** : 0 K, 1 M, 2 C, 4 Y, 16/17/18 Lk/Lm/Lc, 7 et 10 = Or et Rd (à attribuer)
  - ⚠️ CE N'EST PAS DU TON CONTINU (hypothèse « contone » fausse) : le RIP doit séparer en
    9 encres (C Lc M Lm Y K Lk Or Rd) et TRAMER en 2 bits. Prochaines étapes : tables de
    séparation + linéarisation (spectro), tramage (diffusion d'erreur / masque bleu),
    puis émetteur ESC/P2 ; WH/MS : capture d'un travail blanc puis argent
- protocole d'impression : capture demandée à Mathieu (impression ONYX → Epson ; en réseau :
  Wireshark seul, filtre `host <IP Epson>`). Le Comm Driver Windows (epson628187eu.exe) n'imprime
  pas : USBPRINT\EPSONSC-S80600_Serie61AC = classe imprimante USB → /dev/usb/lp0 sous Linux

## État au 24/10 (fin de journée)

Publié : v0.1.0, v0.2.0 (ISO), v0.3.0 (découpe seule, plotter, repérage, multipage —
release en cours de construction). Sur main après v0.3.0 : couleur (profils ICC) — 8a1f114.
Bloqué par le matériel (Mathieu à l'atelier plus tard) : captures USB/réseau Kona (repérage
auto) et Epson (protocole contone), essai réel du Kona (/dev/kona, MGL, carré 100 mm).
Prochaines pistes sans matériel : P4 imbrication (plusieurs travaux sur la laize), mosaïque
(panneaux plus larges que la laize), masquer les tons de découpe dans l'aperçu, Types 2/3
(POSTNET) provisoires, authentification simple.

## P4 — imbrication (24/10, en cours)

- [x] imbrication.rs : placement en rangées (hauteur décroissante, gauche → droite, nouvelle
  rangée quand la laize est pleine), espacement réglable, refus d'un élément plus large que
  la laize ; composition lopdf : objets des PDF sources renumérotés et fusionnés, chaque page
  → formulaire XObject (BBox = MediaBox, Matrix = position), une grande page
- [x] POST /api/planches {laize_mm, espacement_mm, elements:[{id, page, copies}]} → nouveau
  travail « Planche » (mode Imprimer) : rendu, repérage et découpe par le circuit existant
- [x] tests : pas de chevauchement, tout dans la laize ; rendu Ghostscript aux bonnes positions ;
  tracés de découpe décalés ; écran : cases « ajouter à la planche » + laize + copies

## P4 — mosaïque (24/10, en cours)

- [x] mosaique.rs : page plus large que la laize → lés verticaux de largeur ≤ laize, avec
  recouvrement (mm) ; chaque lé = une page (formulaire XObject de la page source, décalé,
  rogné par la MediaBox du lé) ; PDF multipage → circuit ordinaire
- [x] découpe : tracés rognés au rectangle du lé (Liang–Barsky par segment)
- [x] option à l'envoi : « Mosaïque » + laize + recouvrement ; tests : nombre de lés, couleur
  au bon endroit dans chaque lé, recouvrement présent dans les deux lés voisins, tracé coupé
- v0.4.0 publiée le 24/10 (couleur, plaques de découpe signalées, imbrication)

## P4 — réglages enregistrés et dossiers surveillés (24/10, en cours)

- [x] refactor : `Demande` (mode, tracés, repérage, couleur, dpi, mosaïque) + `creer_travail`
  commun à l'envoi web, aux planches et aux dossiers surveillés
- [x] réglages : `<données>/reglages.json`, API GET/POST/DELETE /api/reglages, choix dans le
  formulaire (un réglage remplit les champs)
- [x] dossiers surveillés : `<données>/entrees/<réglage>/` ; scrutation 5 s ; fichier stable
  (taille inchangée entre deux passages) → travail ; déplacé dans `traites/` (ou `refuses/`
  avec la raison) ; partage réseau Samba documenté, pas installé d'office
- [x] tests : réglage appliqué ; fichier déposé → travail ; fichier en cours d'écriture ignoré

## Disque (24/10, en cours)

- v0.4.0 publiée (couleur, imbrication) ; v0.5.0 publiée le 24/10 (mosaïque, réglages,
  dossiers surveillés). Sur main après : purge + espace disque + glisser-déposer (75a1dbb)
- [x] plaques déjà en LZW par défaut (tiffsep) ; composites 32 bits et TIFF d'aperçu effacés (−44 % mesuré)
- [x] purge des travaux plus vieux que `EKO_RIP_CONSERVATION_JOURS` (30 par défaut, 0 =
  jamais), sauf ceux en cours ; passage toutes les heures
- [x] espace libre du dossier de données affiché à l'écran (alerte sous 10 %)

## Contrôle d'accès (24/10, FAIT — v0.6.0, choix validé par Mathieu : « Ok »)

- [x] utilisateurs en CLI (`eko-rip-serveur utilisateur ajouter|supprimer|liste`), mot de
  passe sans écho (rpassword), haché Argon2id (crate argon2), `<données>/utilisateurs.json`
- [x] session : jeton aléatoire 32 octets, cookie HttpOnly SameSite=Strict, 12 h, en mémoire
- [x] /api/* fermé sans session (401), sauf /api/session ; 5 échecs → 5 min de blocage
  par nom ; aucun utilisateur = accès libre + bandeau d'avertissement
- [x] écran : page de connexion, déconnexion ; tests : 401 sans cookie, 200 avec, accès
  libre sans utilisateur, mauvais mot de passe, blocage

## Parité fonctionnelle ONYX / Caldera / SAi Flexi / Fiery XF (24/10)

Consigne de Mathieu (24/10) : « Continue sur tout ce que tu peux continuer sans les
actions de l'atelier », « Interface moderne, documentation, file d'attente… 100 % de
couverture Onyx & Caldera RIP », « regarder du côté de SAI et FieryXF ».
Source : recherche publique du 24/10 (sites éditeurs ; Caldera 403, certains points en
connaissance générale, à confirmer). « 100 % » = les FONCTIONS, pas les 325 pilotes.
Ordre (sans matériel d'abord) :

- [x] Q — **file d'attente par machine** (Kona, Epson) : en attente / imprimer maintenant /
  suspendu, priorité, réordonnancement, historique ; réimpression sans refaire le rendu ;
  sortie Epson = dossier de sortie tant que le pilote n'existe pas (O C S F, P1)
- [x] UI — **interface moderne** : navigation (Travaux, File, Réglages, Couleur, Système),
  cartes avec vignette et badge d'état, détail d'un travail, recherche/filtre ; même page
  pour le client de bureau
- [x] €  — **couverture d'encre et coût** par travail (C CostEstimation, O Hub) :
  % par plaque depuis les TIFF, ml/m² par encre et prix au litre réglables, coût support
  au m² ; export CSV (vers E-KO ERP) (P1)
- [x] W — **blanc automatique** (sous-couche/aplat + rétraction ; argent : ton direct `Silver` déjà pris en charge) : sous-couche générée depuis l'image (seuil), rétraction
  (choke) en mm, inversion ; argent depuis un ton direct ou un masque (O Spot Layer Tool,
  C, F) (P1)
- [x] K — **défonce des tons de découpe** (fait : colorant `None`, ISO 32000-1 § 8.6.6.4) (trouvé le 24/10 sur la plaque blanche) : un
  filet `CutContour` sans surimpression efface l'encre dessous → liseré non imprimé le
  long de la découpe. Retirer les tracés de découpe du PDF imprimé (remplacer leur
  peinture par `n`), pages et formulaires compris. P1.
- [x] D — **documentation complète** FR/EN : guide opérateur (utilisation), guide admin et
  FAQ (administration, 24/10) ; glossaire non fait
- [x] N — imbrication sur la forme du contour (C ContourNesting, S), rotation 90°,
  multi-travaux à l'arrivée (P1, plus lourd)
- [ ] R — règles de dossiers surveillés (préfixe/taille → réglage) (P1)
- [x] (24/10) ordre de découpe (outils puis intérieurs d'abord + plus proche voisin),
  rotation 90° de la planche (4 orientations essayées), repères d'assemblage des lés
- [x] (24/10) preflight : polices non incorporées, images < 100 ppi (alertes, pas de refus)
- [x] P2 ensuite : ~~décalage de découpe~~ (fait 24/10),
  ~~rôles opérateur/admin~~ (fait), ~~notifications~~ (fait 24/10, + durée estimée ; notification système à vérifier dans le client Tauri), ~~motifs step & repeat (demi-saut)~~ (fait), rapports (CSV de consommation ; relevé réel SNMP fait le 24/10 — `releves.rs`, par travail quand le pilote Epson existera), étiquettes Zebra **100 × 150 mm** (réponse de Mathieu le 24/10),
  ~~estimation du temps~~ (fait)
- Avec matériel (atelier) : linéarisation / limites d'encre (spectro), tolérance ΔE des
  tons directs (F), pilote Epson contone, repérage auto Kona, nettoyage de têtes

## Demandes du 24/10 (soir) — Mathieu
« édition rapide (regarder, ajouter des œillets, générer les lay,…) », « Impression de BAT,
imbrication, protocole permettant la collecte des dimensions réelles des médias », « Import
de PDF multi page, import de plus fichiers, mise en pause d'un job, mise en attente d'un
fichier, affichage de l'imposition avant impression, pré visualisation, vérification »,
« Pour les cartouches souvent on a contenance + prix cartouche », « pour les médias
(rouleaux) = prix / m2 ou on saisie la largeur du rouleau + longueur et le prix payé ».
- [x] édition rapide (échelle, rotation, miroir, marge, œillets, contour rectangle) + réédition
- [x] file automatique (suspendu possible), import multiple, visionneuse avec dimensions
- [x] aperçu de l'imposition, BAT PDF — v0.8.0
- [x] largeur du média par SNMP Printer-MIB — VALIDÉ sur la S80600 le 24/10 (IP 192.168.10.114, communauté public) : 1592,7 mm lus en 0,24 s, nom du média non fourni (null) ; rouleau de 1600 mm : 1592,7 = largeur IMPRIMABLE (marges machine, confirmé par Mathieu), bonne laize pour planche et mosaïque
- [x] tarifs : cartouche (contenance ml + prix) → €/l ; rouleaux (€/m² ou largeur + longueur
  + prix payé) → €/m², catalogue de supports, choix du support par travail
- [x] « générer les lay » = les lés de mosaïque : déjà faits (confirmé 24/10)

## Trois chantiers (24/10, Mathieu : « Une fois ok »)
Consigne : « Interface : quelque chose de plus ressemblant à Onyx visuellement mais plus
moderne » ; « OS ISO : on passe de Debian à Ubuntu avec interface … choix : serveur + client
/ serveur seul + interface graphique / serveur seul » ; « Roland VP300 (plusieurs largeurs)
et support de la découpe PAR la machine + Graphtec FC7000-75 ».

### A — Interface façon RIP-Queue, en plus moderne (sans reprendre logos ni icônes ONYX)
- [x] barre latérale : machines (avec leur file), réglages/dossiers surveillés, système
- [x] tableau des travaux (colonnes : vignette, nom, état, format mm, support, prix, date),
  sélection multiple + barre d'outils (imprimer, découper, suspendre, éditer, BAT, relancer,
  supprimer)
- [x] panneau d'inspection à droite (aperçu, pages, plaques, coût, alertes)
- [x] « Importer » en tiroir (panneau d'envoi actuel), thème sombre par défaut + clair

### B — ISO Ubuntu avec bureau, trois profils
- [x] Ubuntu 24.04 LTS (autoinstall/cloud-init au lieu du preseed Debian)
- [x] menu de démarrage : serveur + client (bureau, client Tauri ouvert sur localhost) /
  serveur + bureau (navigateur) / serveur seul (sans bureau)
- [x] éprouvé en VM : CI essai-iso.yml, installation complète du profil serveur seul (vrai paquet, ~6 min) puis démarrage sur disque : page d'état, API client, 403 navigateur (24/10, run 40101059002) ; profils avec bureau : à faire sur un vrai PC

### D — Interface seulement dans les clients (24/10, Mathieu) — FAIT
« le RIP ne doit pas avoir d'interface en ligne que via les clients ; l'URL du serveur doit
seulement amener à une page simple » → signature User-Agent du client Tauri ; page d'état ;
API 403 sans client ; EKO_RIP_ACCES_NAVIGATEUR=1 pour le développement.

### E — ISO bureau : ArtCraft + CUPS (24/10, Mathieu)
« sur la version "desktop" du serveur j'aimerai intégrer "The ArtCraft Suite" afin de
pouvoir éditer certains fichiers (téléchargement / installation durant l'installe de
Ubuntu) ainsi que CUPS »
- [x] image/eko-artcraft : 13 applications, .deb x86_64 de la dernière release officielle,
  SHA256SUMS vérifié (sabotage testé), non bloquant, relançable
- [x] profils bureau : cups, cups-browsed, printer-driver-gutenprint, system-config-printer
- [x] CI : installation réelle des 13 applications sur Ubuntu 24.04 (job artcraft, vert le 24/10)

### C — Machines (FAIT côté logiciel : catalogue, éditeur, HP-GL au pas de la machine ; à éprouver à l'atelier : Graphtec en HP-GL 0,025, Roland découpe, repères ARMS/Roland, impression Roland par capture) : catalogue + Roland VP-300 (famille VP) + Graphtec FC7000-75
- [x] catalogue de machines configurable (machines.json) au lieu d'Epson/Kona codés en dur
- [x] Graphtec FC7000-75 : découpe (GP-GL ou HP-GL), repères ARMS — specs publiques
- [x] Roland VP-300/540 : découpe PAR la machine (CAMM-GL III), repères de recadrage Roland ;
  impression : protocole à capturer si non public
- recherche publique lancée le 24/10

## Plan par phases (à valider)

- [x] P0 — squelette multiplateforme : chaîne PDF → plaques CMJN + tons directs → TIFF
  (profil CMJN par défaut de Ghostscript ; profils ICC par support en P5).
- [ ] P1 — premier pilote d'imprimante : une machine réelle de l'atelier, preuve sur papier.
- [ ] P2 — Mutoh Kona en découpe directe (Linux : `ftdi_sio` + `new_id 0c30 6010`, port
  série `/dev/ttyUSB0` ; groupe `dialout` pour le service).
- [ ] P3 — print & cut : repères, lecture des repères, contours de découpe (CutContour).
- [x] P4 — serveur de travaux : files, imbrication, mosaïque, archivage, interface (clients).
- [ ] P5 — supports : limites d'encre, linéarisation, profils ICC avec spectro (ArgyllCMS).
- [ ] P6 — pilotes supplémentaires, une machine à la fois, selon le parc réel.

« 100 % des 325 pilotes » n'est pas atteignable : beaucoup parlent des protocoles fermés.
On couvre le parc réel, machine par machine.

## 8.6 — Table de tons directs (25/10, en cours)
Mesuré (gs 10.08, tiffsep) : impossible de convertir un ton CHOISI ; `SeparationOrder` sort une
plaque vide pour un ton absent de la page. Donc : rendu inchangé, puis FUSION de la plaque d'un ton
de la table dans C/M/J/N (recette Lab → CMJN calculée par Ghostscript, profil + intention du travail).
- [x] L1 (25/10) Table `tons_directs.json` (nom exact, alias, Lab), API GET/POST, import CSV Lab (`nom;L;a;b`),
      doublon refusé ; sauvegarde + preinst. Jamais de bibliothèque Pantone livrée.
- [x] L2 (25/10 ; écran Système > Tons directs non vu en vrai, à vérifier sur le client) Fusion après rendu (prod + aperçu) ; composite d'aperçu rendu avec l'alternative Lab ;
      noms non trouvés : refus actuel inchangé + journal ; plaque fondue retirée de la page.
- [x] L3 (25/10) Alerte `ton_direct_absent` par page + journal `tons_inconnus.json` (Système, bouton Ajouter).
- [ ] Reportés : ΔE estimé → avec ArgyllCMS (phase 8, 8.4) ; priorité du mode et alerte « PDF sans vrai ton direct » → avec 8.7 (intention par ton).

## ÉCRAN PRINCIPAL façon RIP-Queue (demandé par Mathieu le 25/10, « maintenant », avant 8.11)
Repère : description de la fenêtre principale (tasks/references/onyx-go-25/rip-queue.md § 2.1) et
captures de l'aide publique, regardées pour la DISPOSITION seulement. On s'inspire, on ne reproduit
pas au pixel près : ni textes, ni icônes, ni couleurs, ni captures reprises (consigne du 25/10).
Constat : barre d'outils, menu + machines à gauche, tableau des travaux et inspecteur existent ;
mais Travaux et File sont deux vues séparées, pas d'état machine visible, pas de boutons d'action
sur la file, pas de zone imprimés/journal.
- [x] E1 (25/10 ; volet gauche retiré à la demande de Mathieu : machines et réglages n'apparaissaient deux fois) Vue « Production » unique (remplace Travaux + File, l'ancienne URL y mène) :
      bandeau des machines (état, support chargé, encres en jauges, nb en file) ; tableau
      « À imprimer » (travaux non imprimés + sorties en file ; filtre toutes machines / machine
      choisie) ; tableau « Imprimés » repliable avec compteurs (nombre, poids, % disque = 9.6) ;
      colonne verticale de boutons d'action (monter, descendre, suspendre/reprendre, dupliquer,
      supprimer, propriétés) sur la sélection.
- [x] E2 (25/10 ; « Imbrication » reste au-dessus de la file, pas d'onglet) Inspecteur à onglets à droite : Travail (contenu actuel), Machine (support, encres,
      chauffages, alertes), Réglages (liste + dépôt d'un fichier sur un réglage = import avec ce
      réglage), Imbrication (planche en préparation), Journal (actions et erreurs récentes).
- [x] E3 (25/10 ; déplacement = N × monter/descendre côté serveur) Clic droit par zone, colonnes au choix (clic droit sur l'en-tête), glisser-déposer pour
      réordonner la file, dépôt de fichier partout.
Vérif : captures du client local (navigateur, EKO_RIP_ACCES_NAVIGATEUR=1) à 1440 et 1100 px,
FR/EN, clair/sombre ; tests serveur verts ; puis client Mac.
