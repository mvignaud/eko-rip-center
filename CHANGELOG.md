# Changelog

Toutes les évolutions notables d'E-KO RIP Center. Format inspiré de
[Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versions en
[SemVer](https://semver.org/lang/fr/).
*All notable changes. English summary under each version.*

## [Non publié]

## [0.10.0] — 2026-10-24

### Ajouté
- **Catalogue de machines** (`machines.json`, `GET/POST /api/machines`) : modèle (Epson
  SC-S80600, Mutoh Kona, Graphtec FC7000, Roland VersaCAMM), laize, connexion (série,
  réseau TCP, fichier de périphérique, dossier), adresse SNMP, pas de découpe. Sans fichier :
  l'Epson et le Kona d'origine.
- Découpe vers n'importe quelle découpeuse du catalogue : HP-GL converti au pas de la
  machine (0,01 mm Kona, 0,025 mm par défaut ailleurs), envoi série, réseau ou fichier.
  Graphtec FC7000 et Roland VP : à éprouver à l'atelier (protocoles non publiés en détail).
- Éditeur du catalogue dans Système > Machines (ajouter une Graphtec, une Roland…).
- Répartiteur unique de la file pour toutes les machines ; une imprimante-découpeuse
  (Roland) reçoit l'impression puis la découpe suspendue.

### Modifié
- **Image d'installation : Ubuntu 24.04 LTS** au lieu de Debian, avec trois profils au
  démarrage : serveur + client sur ce PC (petit atelier), serveur + bureau Ubuntu, serveur
  seul. Installation automatique (nom et mot de passe demandés, effacement du disque
  confirmé). Publiée en deux morceaux (limite de 2 Go des releases) avec l'empreinte de
  l'image entière.
- Le client se connecte tout seul au serveur local (`localhost:8640`) au premier lancement
  s'il y en a un.
- **Nouvelle interface façon file de RIP** : barre d'outils (Importer, Imprimer, Découper,
  Éditer, BAT, Relancer, Supprimer sur le travail choisi), barre latérale (vues, machines du
  catalogue avec leur file, réglages enregistrés), tableau des travaux, inspecteur à droite,
  import en fenêtre ; thème sombre par défaut, clair au choix. Les boutons d'impression et
  de découpe et les colonnes de la file viennent du catalogue de machines.
- `GET /api/machines/epson/media` devient `GET /api/machines/{id}/media`.
- **On travaille seulement depuis le client** : un navigateur ouvert sur l'adresse du
  serveur ne voit qu'une page d'état (version, machines, adresse à saisir dans le
  client) et l'API lui répond 403 (`client_requis`). Le client s'annonce par son
  User-Agent (`EKO-RIP-Client/`). `EKO_RIP_ACCES_NAVIGATEUR=1` rouvre le navigateur pour le
  développement. ⚠️ Mettre à jour les clients avec le serveur.

*English: machine catalogue (Epson, Mutoh Kona, Graphtec FC7000, Roland VersaCAMM VP;
serial, TCP, device or folder connection; SNMP; cut step) edited from the client, cuts
converted to each machine's HP-GL step; new RIP-queue style interface (toolbar, machines
sidebar, job table, inspector, dark theme by default); the interface and API are for the
desktop clients only, a browser sees a status page; Ubuntu 24.04 installation image with
three profiles (server + client, server + desktop, server only), published in two parts;
the client connects to a local server by itself.*

## [0.9.0] — 2026-10-24

### Ajouté
- **Prix de revient par travail** (encre + matière), sur la carte et dans le CSV ; prix
  saisis HT ou TTC, taux de TVA, affichage HT ou TTC au choix.
- Encres : contenance et prix de la cartouche → prix au litre (par défaut : cartouche
  Epson 700 ml à 123,90 € TTC, soit 177 €/l).
- **Catalogue de supports** : prix au m², ou largeur + longueur + prix payé du rouleau ;
  choisi à l'envoi ; matière comptée sur toute la laize (par défaut : HEXIS V3000WG,
  1,97 €/m², 1600 mm × 45 m).

*English: cost price per job (ink + media) on the card and in the CSV; ink price per
litre from cartridge size and price; media catalogue (price per m² or roll width, length
and price paid), chosen per job, media counted across the whole roll width; prices
entered and shown excl. or incl. VAT as chosen.*

## [0.8.0] — 2026-10-24

### Ajouté
- **Édition rapide** : échelle, rotation, miroir, marge, œillets, contour de découpe
  rectangulaire ; à l'envoi ou après coup (« Éditer », rendu refait depuis l'original).
- **Mise en file automatique** après le rendu (Epson, Kona), en attente ou suspendue ;
  enregistrée dans les réglages.
- **Largeur du média chargé** lue sur l'Epson par SNMP (Printer-MIB, RFC 3805) et reprise
  comme laize (`EKO_RIP_EPSON_SNMP`, `GET /api/machines/epson/media`) ; à valider sur la
  machine.
- **BAT** (`GET /api/travaux/{id}/bat.pdf?langue=fr|en`) : PDF A4 paysage, image en
  épreuve écran, découpe superposée, réglages, alertes, cadre « Bon pour accord ».
- **Aperçu de l'imposition** de la planche avant de la créer (`POST /api/planches/apercu`) :
  rouleau, éléments placés, longueur et surface utilisée.
- Choix de plusieurs fichiers à la fois ; visionneuse en grand avec dimensions en mm.
- **Pré-vérification** à l'envoi d'un PDF à imprimer : polices non incorporées, images
  de moins de 100 ppi à la taille imprimée (formulaires et images en ligne compris) ;
  alertes affichées sur la carte, sans bloquer le travail.
- **Ordre de découpe** : rainage, perforation et V-cut avant la coupe ; contours
  intérieurs avant ceux qui les contiennent ; à profondeur égale, le plus proche de la lame.
- **Planche : rotation de 90°** (option) quand elle raccourcit la planche ; un élément plus
  large que la laize passe tourné au lieu d'être refusé.
- **Mosaïque : numéro et traits d'assemblage** (option) dans une bande de 10 mm en pied
  de chaque lé.

*English: loaded media width read from the Epson over SNMP (Printer-MIB) and used as
roll width; proofs (BAT) as PDF; imposition preview before building a sheet; quick edit
(scale, rotation, mirror, margin, grommets, rectangular cut contour) on sending or on a
finished job; automatic queueing (on hold if wanted); several files at once; large
viewer; preflight (fonts not embedded, low-resolution images); cutting order; sheet
rotation; panel numbers and joining marks for tiles.*

## [0.7.0] — 2026-10-24

### Ajouté
- **File d'attente par machine** (Kona, Epson) : « Découper (Kona) » et « Imprimer
  (Epson) » mettent une page en file ; chaque machine passe une sortie à la fois, dans
  l'ordre ; en tête, monter, descendre, suspendre, reprendre, annuler ; historique avec
  « Refaire » (sans nouveau rendu) et « Retirer ». File gardée dans `file.json` ; une
  sortie coupée par un arrêt passe en échec au lieu de repartir au milieu.
- Epson, en attendant son pilote : les plaques à imprimer (jamais les tons de découpe)
  sont copiées dans `sorties/epson/`.
- **Blanc automatique** : plaque `White` générée après le rendu, en sous-couche sous
  l'encre (seuil réglable) ou en aplat, avec rétraction en mm ; union avec le blanc du
  fichier. Plaques lues et écrites en flux (bâches de plusieurs Go), TIFF LZW.
- **Encre et coût** : taux d'encre de chaque plaque (mesuré sur l'aperçu), surface,
  encre et coût estimés par page ; tarifs par encre (ml/m² à 100 %, €/l) et support
  (€/m²) dans Système (`tarifs.json`) ; export CSV de la consommation
  (`/api/rapports/consommation.csv`). Les tons de découpe ne comptent pas.
- **Nouvelle interface** : vues Travaux, File d'attente et Système ; panneau « Nouveau
  travail » avec zone de dépôt ; cartes avec vignette et badges d'état ; recherche et
  filtres ; vue file par machine ; affichage téléphone.

### Corrigé
- Un filet `CutContour` sans surimpression effaçait l'encre dessous (liseré non imprimé
  le long de la découpe). Les tons de découpe deviennent le colorant `None` dans le PDF
  imprimé : jamais imprimés, jamais de défonce. La découpe se lit toujours sur l'original.

### Modifié
- Les messages s'affichent dans un bandeau flottant, visible depuis toutes les vues.
- L'écran ne se reconstruit plus que si les données changent (un clic ne tombe plus sur
  un bouton remplacé).
- Guides d'utilisation FR/EN réécrits (file d'attente, nouvelle interface, glossaire) ;
  le pas HP-GL indiqué est corrigé (0,010 mm).
- Supprimer un travail efface aussi ses sorties en dossier ; refusé tant qu'une sortie
  l'attend dans la file.

### Retiré
- Route `POST /api/travaux/{id}/pages/{page}/decouper` : remplacée par `POST /api/file`.

*English: per-machine output queue (Kona, Epson; to top, up, down, hold, resume,
cancel, redo without re-rendering, history; Epson plates copied to `sorties/epson/`
until its driver exists), new interface (Jobs, Queue, System views, drop area, cards
with thumbnails and state badges, search and filters, phone layout), ink coverage and
cost estimate per page with editable rates and CSV export, automatic white (underlay or
flood, choke, union with the file's own white, streamed for multi-GB plates), cut spot
colours no longer knock out the ink beneath them (rendered as the PDF `None` colorant),
rewritten FR/EN usage guides.*

## [0.6.0] — 2026-10-24

### Ajouté
- **Contrôle d'accès** : comptes créés sur le serveur (`eko-rip-serveur utilisateur
  ajouter|supprimer|liste`), mots de passe hachés en Argon2id, session de 12 h par cookie
  `HttpOnly`, blocage 5 min après 5 essais faux. Écran de connexion et bouton « Se
  déconnecter ». Sans compte déclaré, l'accès reste libre et un bandeau rouge le signale.
- **Purge automatique** des travaux plus vieux que `EKO_RIP_CONSERVATION_JOURS` jours
  (30 par défaut, 0 = jamais), toutes les heures ; un travail en cours n'est jamais
  purgé. ⚠️ À la mise à jour, les travaux de plus de 30 jours sont effacés au démarrage.
- **Glisser-déposer** : un ou plusieurs fichiers lâchés sur la page partent avec les
  réglages du formulaire (écran web et client de bureau).
- Pied de page : version, espace libre du disque des données (en rouge sous 10 %),
  durée de conservation. Route `/api/systeme`.

### Modifié
- Les composites CMJN 32 bits et les TIFF d'aperçu sont effacés après le rendu (≈ 40 % de
  place en moins) ; les plaques de production restent, compressées en LZW.

*English: access control (server-side accounts `eko-rip-serveur utilisateur
ajouter|supprimer|liste`, Argon2id hashes, 12 h HttpOnly session, 5-minute lock after 5
wrong attempts, sign-in screen; open access with a red banner while no account exists),
automatic purge of old jobs, drag and drop, disk space in the footer, smaller job
folders.*

## [0.5.0] — 2026-10-24

### Ajouté
- **Mosaïque** : option à l'envoi ; une page plus large que la laize est coupée en lés de
  même largeur avec recouvrement réglable (20 mm par défaut), un lé par page ; l'original
  est gardé (`source-original.pdf`). La découpe de chaque lé est rognée à son bord.
- Les tracés de découpe sont rognés à la page : la lame ne part jamais hors du support.
- **Réglages enregistrés** : « Enregistrer ces réglages… » garde le formulaire sous un nom
  (mode, tracés, repérage, couleur, résolution, mosaïque) ; le choisir remplit le
  formulaire.
- **Dossiers surveillés** : chaque réglage a son dossier `<données>/entrees/<nom>/` ; un
  fichier déposé (PDF, AI, SVG), une fois sa copie terminée, devient un travail avec ce
  réglage ; un fichier refusé part dans `refuses/` avec la raison.

*English: tiling (panels of equal width with overlap when the page is wider than the
roll, cut clipped per panel), cut paths clipped to the page, saved presets and hot
folders (`entrees/<preset>/`, rejected files moved to `refuses/` with the reason).*

## [0.4.0] — 2026-10-24

### Ajouté
- **Gestion des couleurs** : profils ICC déposés depuis l'écran (en-tête vérifié, rangés
  dans `<données>/profils/`, jamais livrés avec le logiciel) ; par travail, profil
  d'entrée CMJN (par exemple Epson Wide Gamut), profil d'entrée RVB, profil du support
  en sortie, intention de rendu, option « ignorer les profils du PDF ». L'aperçu
  composite simule le support (épreuve écran). Les tons directs ne sont pas convertis.
- **Imbrication (planche)** : cocher des pages de travaux et leur nombre de copies, choisir
  la laize et l'espacement, puis « Créer la planche » ; les éléments sont placés en rangées
  sur la laize, et la planche devient un travail ordinaire (rendu, repérage, découpe de tous
  ses contours). Un élément plus large que la laize est refusé avec sa largeur.
- Le CI vérifie la syntaxe JavaScript des pages.
- Les plaques de découpe (`CutContour`…) sont signalées « découpe, non imprimée » ; la
  liste par page (`plaques_decoupe`) servira au pilote d'impression pour les exclure.

*English: colour management (ICC profiles uploaded from the screen, CMYK/RGB input,
media output, rendering intent, soft-proof preview), cut plates flagged as not printed,
nesting (several jobs and copies on the roll width as one sheet).*

## [0.3.0] — 2026-10-24

### Ajouté
- **Découpe seule** : un fichier AI (enregistré « compatible PDF »), PDF ou SVG est
  découpé sans être imprimé. Choix des tracés : tons de découpe (`CutContour`…, ou en
  SVG le nom du calque ou du tracé) ou tous les tracés. Un SVG ne peut être que découpé.
- **Envoi au plotter** depuis l'écran (« Envoyer au plotter »), avec son état. Le Mutoh
  Kona, branché en USB, est reconnu par Linux comme un port série (puce FTDI) : le
  paquet installe une règle udev qui le présente en `/dev/kona` ; réglages du port
  dans `/etc/default/eko-rip-serveur`.
- Aperçu des tracés à découper dans la fiche du travail.
- **Repérage Kona à l'impression** : croix P1–P4 (repérage manuel) ou cadre Mutoh de
  type 1 (lecture automatique, sans code-barres ni CutServer), aux cotes du manuel
  Kona ; la page est agrandie, l'image décalée, les repères en noir pur, et les tracés
  de découpe suivent le même décalage.

- **PDF multipages** : chaque page est rendue avec ses propres plaques (une page sans ton
  direct n'a pas sa plaque), reçoit ses repères, et a son fichier de découpe et son
  envoi au plotter. API par page : `/api/travaux/{id}/pages/{n}/…`.
- **Supprimer** un travail terminé ou en échec, **relancer** un travail en échec (les
  sorties précédentes sont effacées avant le nouveau rendu).

### Modifié
- **HP-GL au pas de 0,010 mm** (100 unités par mm) au lieu de 0,025 mm : c'est le seul
  pas accepté par la découpe de contour du Kona. Régler le menu « Resolution » du Kona
  sur 0,010 mm.

*English: cut-only mode (AI, PDF, SVG; cut spot colours or all paths), send to the
Mutoh Kona from the screen (USB FTDI serial port, udev rule, /dev/kona), Kona
registration at print time (P1–P4 crosses or Mutoh type 1 frame), multi-page PDFs,
delete/retry jobs, HP-GL at 0.010 mm per step.*

## [0.2.0] — 2026-10-23

### Ajouté
- **Image d'installation du serveur** (`eko-rip-center-serveur-<version>-amd64.iso`) :
  l'installateur officiel Debian 13 avec une entrée « Installer E-KO RIP - EFFACE LE
  DISQUE » en tête des menus (démarrage BIOS et UEFI). L'installation est automatique :
  français, nom `eko-rip`, disque entier, SSH, serveur E-KO RIP Center installé et
  démarré. Seuls le mot de passe du compte `eko` et la confirmation d'écriture du
  disque sont demandés ; l'adresse de l'écran s'affiche à la connexion de la console.
- **Découpe** : les tracés peints dans un ton de découpe (`CutContour`, `Thrucut`,
  `Kisscut`, `Insidecut`, `Perforation`, `Creasing`, `Score`, `Vcut` ; casse et espaces
  ignorés) sont extraits du PDF, formulaires et transformations compris, puis écrits en
  HP-GL (1/40 mm, un outil par type de ton). Le fichier `.plt` se télécharge depuis
  l'écran. Éprouvé sur une planche de 48 stickers réelle.

### Corrigé
- La release ne joint plus le journal de construction de l'image Docker.
- Le paquet Debian du serveur a une description.

*English: cut paths (CutContour, Thrucut, Kisscut…) extracted from the PDF and written
as HP-GL, downloadable from the screen; server installation image (Debian 13 installer with an automatic
“E-KO RIP” entry, BIOS and UEFI; only the account password and the disk write
confirmation are asked; wipes the disk). Release no longer attaches the Docker build
record.*

## [0.1.0] — 2026-10-23

Première version : la chaîne PDF → plaques d'encre fonctionne, sans pilote de machine.

### Ajouté
- **Serveur** : reçoit un PDF depuis le navigateur (contrôle de l'en-tête, de la
  taille et de la résolution), le rend avec Ghostscript en plaques 8 bits :
  CMJN plus une plaque par ton direct. Le blanc (`White`) et l'argent métallisé
  (`Silver`) sortent sur leur propre plaque.
- **Écran web** : envoi, liste des travaux, aperçu composite et aperçu de chaque
  plaque ; français et anglais.
- **Client de bureau** (macOS, Windows, Linux) : retient l'adresse du serveur et
  ouvre son écran ; menu « Changer de serveur ».
- **Paquets** : image Docker, paquet Debian avec service systemd, binaire Linux.

### Limites connues
- Aucun pilote d'imprimante ni de plotter : les plaques ne partent pas encore vers
  les machines.
- Première page seulement d'un PDF multipage (corrigé après la 0.2.0).
- Pas d'authentification : le serveur ne doit être joignable que depuis le réseau
  de l'atelier.
- Paquets du client non signés : macOS et Windows afficheront un avertissement.

*English: first release. PDF to ink plates (CMYK + one plate per spot colour, White
and Silver included), web screen in French and English, desktop client, Docker image,
Debian package. No printer or cutter driver yet.*
