# Changelog

Toutes les évolutions notables d'E-KO RIP Center. Format inspiré de
[Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versions en
[SemVer](https://semver.org/lang/fr/).
*All notable changes. English summary under each version.*

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
