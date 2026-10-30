# Changelog

Toutes les évolutions notables d'E-KO RIP Center. Format inspiré de
[Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versions en
[SemVer](https://semver.org/lang/fr/).
*All notable changes. English summary under each version.*

**Numérotation (décision du 25/10/2026)** : `X.Y.Z` — **X** version majeure, chacune avec un
nom tiré de l'imprimerie (Gutenberg, Garamond, Cyan, Magenta, Dorure…) ; **Y** fonction,
pilote ou correctif ordinaire ; **Z** correctif de sécurité seulement. La **1.0.0**, première
version nommée, sortira quand le RIP couvrira 100 % des fonctions de la feuille de route de
parité ; d'ici là, 0.Y.Z.
*Versioning: X major (named after printing), Y features/drivers/fixes, Z security fixes only;
1.0.0 when feature parity is reached.*

## [Non publié]

### Ajouté
- **Épreuve TIFF après RIP** (feuille de route 9.13) : bouton **Épreuve TIFF** et route
  `/api/travaux/{id}/pages/{n}/epreuve.tif` ; TIFF des plaques rendues en valeurs machine
  (CMJN, profil du support) ou colorimétrique (RVB), tons directs gardés en canaux,
  aplatis ou supprimés, plafond de taille (la définition baisse, la taille physique reste
  exacte, pour certifier les cotes d'un lé). Réglage `epreuve` pour la faire d'office
  après le rendu, et **épreuve seule** (`epreuve_seule`) : rien ne part à l'impression.
- **Envoi avec un réglage enregistré** : champ `reglage` de `POST /api/travaux` ; le
  serveur applique lui-même le réglage nommé (refus `reglage_inconnu` sinon). Base des
  nouveaux clients natifs Mac et Linux (`docs/clients-natifs.md`).

*Added: TIFF proof after RIP (device CMYK or colorimetric RGB, spot colours as channels,
flattened or removed, size cap keeping exact physical size), made on demand or after
rendering, and a proof-only option that never prints. Upload with a saved preset
(`reglage` field), the base of the new native Mac and Linux clients.*

## [0.28.0] — 2026-10-28

### Ajouté
- **Canal alpha → plaque blanc ou vernis** (feuille de route 9.12) : réglage « Canal alpha »
  de la section Blanc ; le détourage d'un PNG ou d'un TIFF donne une plaque `White` ou
  `Varnish` qui suit le masque alpha, posée en surimpression.

*Added: alpha channel of a PNG/TIFF → White or Varnish plate following the cut-out.*

## [0.27.0] — 2026-10-28

### Ajouté
- **Estimation d'un dossier pour l'ERP** (pont, lot R4 ; feuille de route 7.6) : à la
  demande de l'ERP, le RIP répond par `travail.estimation` avec les chiffres de la fiche des
  travaux rendus du dossier, copies comprises (surface, encre par plaque, durée), et la
  longueur de rouleau de toutes les copies imbriquées sur la laize chargée. Aucun montant.
  Pour un devis, l'ERP peut aussi envoyer un fichier à estimer : le RIP le rend à part (ni
  liste ni file), l'estime de la même façon, puis l'efface.
- **Actions de l'ERP sur la file** (pont, lot R3.2 ; feuille de route 7.7) : l'ERP peut
  suspendre, reprendre, annuler, passer en tête ou changer l'échéance d'un dossier
  (`ref_erp`). Le RIP l'applique aux sorties encore en attente ou suspendues de ses travaux,
  laisse telles quelles celles déjà lancées, et répond par `travail.action` ; un dossier déjà
  imprimé ne s'annule pas, il se signale (`deja_imprime`).
- **Réception des travaux de l'ERP** (pont, lot R3 ; feuille de route 7.3) : le RIP télécharge
  les fichiers que l'ERP lui envoie (signés, SHA-256 vérifié), en fait des travaux au réglage
  enregistré désigné, avec la référence du dossier, les copies et l'échéance dans la fiche,
  et l'accuse à l'ERP ; un envoi impossible part en `travail.refuse` avec son code. Un envoi
  reproposé ne fait jamais deux travaux.
- **Dépôt APT des postes Linux** sur `https://apt.e-ko.fr/rip/` : installation par
  `wget -qO- https://apt.e-ko.fr/rip/installer | sudo sh`, puis mises à jour par
  `sudo apt update && sudo apt upgrade`. Publication : `outils/publier-depot.sh`.
- **Bascule des encres claires par mode** (feuille de route 8.11) : trois préréglages à
  nous, douce, équilibrée et franche, et l'encre totale du mode (`/ franche / 260 %`),
  appliqués par le pilote direct de l'Epson.
- **Limite d'encre totale proposée** à partir d'aplats mesurés (L*), à valider à la main,
  et **mire de linéarisation CGATS** resserrée dans les hautes lumières.
- Feuille de route de parité et notes rangées dans `docs/feuille-de-route/`.
- **Dossier automatique `entrees-auto/`** (feuille de route 9.1) : le réglage vient de la
  première règle qui prend le fichier, par motif de nom (`BACHE_*`) et format de la première
  page d'un PDF ; règles éditées dans Système et gardées dans la sauvegarde.
- **Tons directs des TIFF** (feuille de route 9.2) : chaque canal en plus devient sa
  plaque, en surimpression ; `Spot1`, `White` ou `Blanc` donne la plaque de blanc ; les
  canaux de sélection de Photoshop sont ignorés. Sens des valeurs à confirmer sur un
  fichier de l'atelier.
- **Une page par travail** et **Refuser si une police manque** (feuille de route 9.3),
  cases du panneau d'envoi gardées dans le réglage : un PDF de plusieurs pages devient un
  travail par page (« fichier (p. 2/5) ») ; un fichier dont une police n'est pas
  incorporée, hors 14 polices de base, est refusé (`polices_manquantes`).
- **Ports IPP et renvoi** (feuille de route 9.4) : une machine peut sortir par IPP
  (Print-Job brut vers une file CUPS ou une imprimante, adresse `ipp://…` montrée à
  l'écran) ou par renvoi vers un autre E-KO RIP Center, qui passe le flux à sa machine
  (jeton `eko-rip-serveur renvoi jeton`, jamais réaffiché). L'Epson imprime en direct par
  les deux.
- **Documentation** (feuille de route 9.7) : glossaire FR/EN (`docs/fr/glossaire.md`,
  `docs/en/glossary.md`), aide par machine (Epson, Kona, Graphtec, Roland, connexions :
  `machines.md`) et exigences du poste serveur (SSD dédié aux données, onduleur, IP fixes,
  réseau) dans l'installation du serveur.
- **Compteurs de file** (feuille de route 9.6) : colonne **Tampon** des machines (poids des
  plaques à envoyer) et, sous le tableau, tampon total et % du disque occupé ; `tampon` dans
  `/api/systeme`, `poids` de chaque sortie à passer dans `/api/file`.
- **Épreuve sur une 2e machine** (feuille de route 9.5) : liste **BAT sur…** de la page ;
  la copie « nom-BAT » part sur la machine d'épreuve, l'original attend suspendu
  (« Attend le BAT ») jusqu'à **Reprendre**.
- **Motif à balises du nom** (feuille de route 9.10) pour `entrees-auto/` :
  `{nom}__{machine}__{reglage}__{copies}` lit dans `Affiche__epson__bache__30.pdf` le
  réglage, la machine et les copies ; aussi `{support}`, `{repetition}`, `{laize}`,
  `{echelle}`, `{largeur}`, `{hauteur}`. Un nom hors motif passe par les règles avec une
  alerte, jamais refusé en silence ; motif édité dans Système (`/api/balises`), gardé dans
  la sauvegarde.
- **Rendus en parallèle** (feuille de route 9.11) : plusieurs travaux rendus à la fois (un
  par cœur, 4 au plus, `EKO_RIP_RENDUS`) ; les arrivées des dossiers surveillés et de l'ERP
  partent en file dans leur ordre de dépôt, un envoi depuis l'écran passe devant elles.
  **Rendus simultanés** dans Système, `rendus` dans `/api/systeme`.
- **Très grands formats** (feuille de route 9.16) : une page dont le composite de
  Ghostscript dépasserait 3 Go (bâche 1,6 × 3 m à 720 dpi) est rendue par bandes, puis
  ses plaques sont recollées point pour point ; une plaque de plus de 3,5 Go est écrite en
  BigTIFF. Le rendu de ces pages échouait au-delà de 4 Go.

*Added: signed online APT repository for Linux clients; `outils/publier-depot.sh` publishes. Per-mode light-ink
switch presets and total ink limit; proposed ink limit from measured patches; highlight-dense linearisation chart. Automatic hot folder with name-pattern and page-size rules. TIFF spot colour channels as plates. One page per job, and refuse files with missing fonts, as preset options. IPP output port and forwarding to another E-KO RIP. Proof copy on a second machine, original held. Queue counters: spool weight per machine, disk use. FR/EN glossary, help by machine, server PC requirements. Tagged file name pattern for the automatic folder (preset, machine, media, copies, repetition, roll width, scale, size). ERP bridge: jobs sent by the ERP (signed download, SHA-256 checked), ERP actions on the queue (hold, resume, cancel, priority, due date) and estimates of an order or of a file before sending (area, ink per plate, duration, nested roll length; no amounts). Parallel rendering (`EKO_RIP_RENDUS`), queued in arrival order, manual uploads first. Very large pages rendered in bands and joined (BigTIFF over 3.5 GB) instead of failing past 4 GB.*

## [0.26.0] — 2026-10-25

### Corrigé
- **TIFF de Photoshop** : quand l'en-tête TIFF donne une résolution sans unité, la vraie est
  lue dans les données de Photoshop (une affiche de 891 × 1260 mm à 300 ppi sortait à
  3,7 × 5,2 m, et son rendu dépassait le délai).
- Une transparence partout opaque (PNG, TIFF) ne devient plus un masque : rendu plus rapide.

*Fixed: Photoshop TIFF resolution read from its image resources when the TIFF header has no
unit; fully opaque alpha no longer becomes a soft mask.*

## [0.25.0] — 2026-10-25

### Ajouté
- **Progression de l'envoi** : un cadre (bas droite) donne le nom du fichier, une barre, les
  Mo envoyés, le pourcentage et le temps restant, puis indique la conversion côté serveur ;
  un gros fichier (plusieurs Go) ne laisse plus l'écran sans réaction.

*Added: upload progress panel (file name, bar, MB, %, time left, then server-side conversion).*

## [0.24.0] — 2026-10-25

Images en entrée. Publiée sans la CI.

### Ajouté
- **Images TIFF, PNG et JPEG** acceptées (envoi, dossiers surveillés) : converties en PDF
  d'une page à leur taille réelle (résolution du fichier ; sans résolution : 72 ppi, signalé),
  écrit en flux sans charger l'image entière ; CMJN, RVB ou gris, 8 ou 16 bits, profil ICC
  repris, transparence en masque (pas d'encre), JPEG repris tel quel. Refus avec un message :
  TIFF à palette, YCbCr, Lab ou à plans séparés, PNG entrelacé. Canaux de tons directs d'un
  TIFF ignorés et signalés pour l'instant.

### Modifié
- Taille maximale d'un fichier reçu : 8 Go par défaut (`EKO_RIP_MAX_MO=8192`).

*Added: TIFF, PNG and JPEG input, streamed into a one-page PDF at real size (CMYK/RGB/grey,
8/16 bits, ICC kept, transparency masked, JPEG passed through). Changed: 8 GB upload limit.*

## [0.23.0] — 2026-10-25

Vue Production : clic droit, colonnes, glisser-déposer. Publiée sans la CI.

### Ajouté
- **Clic droit** : sur une ligne (actions de file de la sortie, puis celles de la barre
  d'outils), sur une machine (n'afficher qu'elle, son état, relire l'Epson, la configurer),
  sur l'en-tête d'un tableau (colonnes à montrer ou masquer, gardées par le poste).
- **Glisser-déposer** dans « À imprimer » : une sortie en attente ou suspendue prend la place
  d'une autre de la même machine.

*Added: right-click menus (rows, machines, headers), column choice kept per computer,
drag and drop to reorder the queue.*

## [0.22.0] — 2026-10-25

Écran principal refait : une seule vue de production. Publiée sans la CI ; l'écran est servi
par le serveur, les clients en place restent valables.

### Modifié
- **Vue Production** (remplace Travaux et File d'attente) : machines en haut (état, support
  chargé, encres de l'Epson, file, durée ; un clic filtre sur la machine), **À imprimer** (la
  file dans son ordre, puis les travaux non envoyés, avec une colonne d'actions de file :
  en tête, monter, descendre, suspendre, reprendre, annuler, refaire, restaurer, retirer),
  **Imprimés** repliable (fin, relevé d'encre et de support).
- **Inspecteur à onglets** : Travail, Machine, Réglages (déposer un fichier sur un réglage
  l'importe avec ce réglage), Journal (messages de l'écran).
- Barre du haut : Production et Système remplacent le volet gauche ; sous 1360 px, les
  outils gardent leur symbole et leur nom passe en bulle.

*Changed: single Production view (machines on top, to-print queue with action buttons,
collapsible printed list), tabbed inspector (Job, Machine, Settings with drop-on-setting
import, Log), navigation moved to the top bar.*

## [0.21.0] — 2026-10-25

Pont ERP (canal sortant) et tons directs. Publiée sans la CI ; le client macOS 0.19.0 et les
clients 0.16.0 restent valables (l'écran est servi par le serveur).

### Ajouté
- **Pont ERP, lot R2** : journal des événements numérotés (rejouables après coupure), canal
  sortant HTTPS vers l'ERP (une requête par seconde au plus, `429` suivi de `Retry-After`),
  catalogue (machines, supports, modes, préréglages, découpeuses), états des travaux et des
  sorties, consommation réelle en fin d'impression et de découpe, et **état des machines**
  (`machine.etat` : encres, rouleau, support chargé, chauffages, alertes). Aucun montant.
- **Table des tons directs** (Système > Tons directs) : nom exact, alias, couleur Lab ; import
  du nuancier Lab d'un fournisseur (CSV `nom;L;a;b`). Un ton de la table est imprimé en CMJN
  d'après sa couleur Lab (profil de sortie du travail), l'aperçu la montre et le travail le
  signale. **Intention propre aux tons directs** : celle du serveur, ou celle du réglage ou du
  travail.
- Un ton direct ni encre, ni découpe, ni dans la table est signalé dès le rendu, page par
  page, et noté dans Système (« Ajouter » le fait entrer dans la table).

### Corrigé
- Contrat du pont : après une coupure, un travail en cours passe en échec (`interrompu`).
- Deux tests instables stabilisés (rafale du pont sous charge, réglages de découpe globaux).

*Added: ERP bridge outbound channel (numbered events, catalogue, job/output states, real
consumption, machine state; rate-limited), spot colour table (exact names, aliases, Lab, CSV
import) printed in CMYK from Lab with its own rendering intent, missing spot colours flagged
per page and logged. Fixed: interrupted jobs reported as failed; two flaky tests.*

## [0.20.0] — 2026-10-25

Finitions de l'éditeur, découpe complète, imbrication automatique, consommation réelle.
Publiée sans la CI ; client macOS 0.19.0 et clients 0.16.0 toujours valables.

### Ajouté
- **Finitions** (éditeur et envoi) : traits de coupe, croix, contour, repères de pli et de
  couture (styles de trait), fond perdu (couleur, premier pixel, miroir, duplication), œillets
  par quantité ou espacement (4 formes), étiquette en marge, QR code avec balises ; annuler /
  rétablir (50 pas) ; filtres couleur par travail (courbes C M J N, luminosité, saturation,
  avant/après, export vers un réglage) et remplacement de couleur.
- **Découpe** : préfixes de découpe réglables ; une couche par nom de ton, avec le réglage de
  coupe du même nom ; ordre et activation des couches, enregistrés ; alertes de contrôle
  préalable ; coupe seulement après l'impression ; pré-contrôle de la découpeuse avant
  d'imprimer ; mise en coupe (rotation, flèche d'avance ; en découpe seule : miroir, échelle,
  copies X/Y) ; contour automatique, retrait, décalage, extension de l'image près des tracés ;
  tracés ouverts et tri ; coupe dès l'arrivée et archive après coupe ; détourage automatique ;
  fond sous repères, compensation de cintrage, variantes de machine. Repérage Mutoh : un cadre
  simple de 1,5 m et plus devient multi-cadre.
- **Imbrication automatique dans la file** : stratégies (un par un, groupé, économie),
  déclencheurs (minutes, inactivité, m², longueur cible ou maxi, rangée pleine), regroupement
  par support, mode, découpeuse et réglages de coupe, planche en préparation visible dans la
  file, contour de planche et marges, purge, justification ; **fiche travail** (commande,
  client, quantité, échéance = priorité) ; **pages à ne pas imprimer** ; une planche abandonnée
  rend ses travaux à la file et ne peut plus être relancée (pas de double impression).
- **Consommation réelle par impression** : encre et support relevés au début et à la fin
  (« < 7 ml » sous la précision de l'imprimante), gâche, journal des impressions (90 jours),
  rapport CSV du réel et rapport CSV des estimations, facteurs de correction proposés et
  appliqués sur validation, encre estimée en comptant les points du tramage.

### Corrigé
- Les réglages de découpe du serveur (`decoupe.json` : préfixes, options) sont repris par la
  sauvegarde de la configuration et par la copie faite avant une mise à jour.
- Textes de l'écran : origine Y de l'Epson comptée après la tête vide de 20 mm ; note de la
  connexion Dossier de l'Epson (le pilote direct existe : connexion Réseau) ; « Reprendre »
  au lieu d'un bouton « Couper maintenant » inexistant.

### Documentation
- Doc d'utilisation (FR, EN) et d'administration à jour de tout ce qui est livré depuis la 0.14 :
  supports et modes, pilote direct de l'Epson, éditeur visuel, lés et tuilage, découpe.

*Added: finishing (marks, bleed, grommets, label, QR, undo, colour filters), full cutting
workflow (layers by name, pre-checks, cut after print, transforms, contours, auto-path, multi
frame from 1.5 m), automatic nesting in the queue with job ticket and excluded pages, actual
consumption per print with journal, reports and correction factors.*

## [0.19.0] — 2026-10-25

**Premier tirage par notre propre pilote de l'Epson**, et l'éditeur visuel prend forme.
Publiée sans la CI ; les clients de bureau de la 0.16.0 restent valables.

### Ajouté
- **Pilote direct de l'Epson SC-S80600** (connexion « Réseau », port 9100) : rendu, séparation
  des encres (couples clair/foncé, limite d'encre), tramage 4 tailles de point, flux ESC/P2
  envoyé au fil des bandes ; état suivi par SNMP pendant l'impression, échec si l'imprimante
  est en alerte (capot ouvert…). Premier aplat CMJN imprimé le 25/10, couleurs justes. La
  sortie « Dossier » reste disponible. Tête vide de 20 mm et plafond de 85 % par canal, contre
  les bandes observées au premier tirage (à confirmer au second).
- **Éditeur visuel** (fiche du travail) : vignettes des pages, sélection, couches activables
  (image, découpe en magenta, blanc, argent) ; **recadrage** à poignées et par champs (un cadre
  hors de la page est refusé) ; **taille** proportionnelle ou libre, largeur ou hauteur
  imposée ; **rotation et miroir**, image seule ou image et découpe ; badge « Miroir » dans la
  file.
- **Lés et tuilage** : lés dessinés, frontières déplaçables (lés inégaux), colonnes × rangées,
  zone de soudure sans encre, coupe au milieu du recouvrement, tuiles à ne pas imprimer,
  recouvrement propre à une tuile, ordre d'impression des tuiles.

### Modifié
- La découpe applique la boîte (BBox) des formulaires du PDF, comme le veut la norme.
- L'image d'installation n'est plus publiée sur GitHub : un fichier par version sur le nuage
  d'E-KO (lien dans l'installation du serveur).

*Added: direct Epson SC-S80600 driver (first print on 25/10), visual editor (thumbnails,
selection, layers, crop, size, rotation/mirror), tiles (unequal, rows × columns, weld zone,
per-tile overlap, print order). Changed: form BBox applied to cuts; ISO moved to the E-KO cloud.*

## [0.18.0] — 2026-10-24

Fin de la phase « supports et modes » de la feuille de route. Publiée sans la CI ; les
clients de bureau de la 0.16.0 restent valables.

### Ajouté
- **Paquet support** : exporter un support, ses modes et les réglages qui s'en servent en
  un seul fichier, et l'importer sur un autre serveur (identifiants conservés, réimport sans
  doublon ; refus d'un paquet fait par une version plus récente). Sans profil ICC.
- **Export / import de réglages** choisis (panneau Sauvegarde) ; si le nom existe déjà :
  renommer (« nom 2 ») ou écraser.
- **Pool d'imprimantes** : un même nom de pool sur des machines identiques (même modèle,
  même laize) ; une impression envoyée au pool part sur la première machine prête ; refus
  si les supports chargés diffèrent.

*Added: media package export/import, preset export/import (rename or overwrite), printer
pools.*

## [0.17.0] — 2026-10-24

Fin de la phase « socle » de la feuille de route, et support chargé dans les machines.
Publiée sans la CI (construite sur le serveur de test) ; les clients de bureau ne changent
pas : ceux de la 0.16.0 restent valables.

### Ajouté
- **Support et mode du travail** : à l'envoi, « Support et mode d'impression » ; le mode
  impose la résolution, le support relie aux tarifs ; changer de mode refait le rendu.
- **Support chargé** dans l'Epson, reconnu par son n° de support lu sur l'imprimante, ou
  saisi par l'opérateur (Système) ; **éditeur des supports** de l'Epson (nom, n° dans
  l'imprimante, laize, encres, modes).
- **Attente de support** : une impression attend que le bon support soit chargé, et part
  d'elle-même ensuite ; la laize du rouleau chargé borne la page.
- **Raison d'attente** affichée pour chaque sortie (retenue, machine injoignable, fichier
  manquant, trop large, autre support) ; une sortie bloquée laisse passer les suivantes.
- **Seuil d'espace libre** (`EKO_RIP_SEUIL_LIBRE_MO`, 5 Go par défaut) : purge des plus
  vieux travaux inactifs, et refus anticipé d'un travail qui ne tiendrait pas.
- **Sauvegarde automatique** de la configuration avant chaque mise à jour du paquet
  (`/var/lib/eko-rip/sauvegardes/`).

### Corrigé
- Un dossier surveillé ou de sortie supprimé est recréé et signalé, au lieu d'être ignoré.
- Page d'un travail en cours de rendu : « pas prêt » (409) au lieu de « introuvable ».

*Added: job media and print mode, loaded media (read from the Epson or set by hand), media
editor, wait-for-media, per-output wait reason, free-space threshold, automatic config
backup before package updates. Fixed: missing folders recreated, 409 while rendering.*

## [0.16.0] — 2026-10-24

### Ajouté
- **Pont ERP, lot R1** — contrat `eko-rip-erp/1`, décrit dans `docs/fr/pont-erp.md` :
  - identifiants immuables (`uuid`) des machines, couches de découpe et réglages
    enregistrés, attribués par le serveur ; un identifiant inventé est refusé ;
  - accès machine à machine : jeton de l'ERP créé par `eko-rip-serveur erp jeton`, gardé
    en empreinte seulement, valable sur les seules routes `/api/erp/` (fermées sans lui,
    même en accès libre) ;
  - secret HMAC (`eko-rip-serveur erp secret`) et format de signature des événements
    (`X-EKO-RIP-Signature: t=…,v1=…`) ;
  - modèle d'états publié, lisible par programme : `GET /api/erp/contrat`.

*Added: ERP bridge lot R1 (contract `eko-rip-erp/1`): immutable identifiers, ERP token
limited to `/api/erp/`, HMAC event signature format, published state model.*

## [0.15.0] — 2026-10-24

### Ajouté
- **Grand aperçu façon éditeur** (fiche du travail) : zoom de 25 à 800 % à la molette (le
  point sous le curseur reste fixe), boutons Ajuster et 100 %, outils Main et Loupe (clic,
  Maj+clic, rectangle), règles en mm sur deux bords qui suivent le zoom, position du curseur
  en mm, **laize** visible (média lu sur l'Epson, sinon laize de la machine) avec la partie
  qui dépasse en rouge, calque de découpe superposable ; clavier : + − 0 1 et flèches.
- **Émetteur ESC/P2 de l'Epson SC-S80600** (pas encore branché sur la file) : en-tête,
  réglages 720 × 720, point variable, bandes de 16 lignes en 2 bits compressées PackBits, neuf
  canaux ; contrôlé contre la structure d'un flux reçu par notre machine. Premier essai papier
  à venir.

*Added: editor-style large preview (zoom, hand, magnifier, mm rulers, roll width, cut
overlay, keyboard). Epson SC-S80600 ESC/P2 emitter (not yet wired to the queue).*

## [0.14.0] — 2026-10-24

### Ajouté
- **Supports et modes d'impression par machine** (`supports.json`) : un support porte son
  nom, sa configuration d'encres et sa laize ; un mode porte sa résolution, son nombre de
  passes et son chauffage. Chacun a un identifiant stable, distinct de son nom (renommer ne
  le change pas ; un identifiant inventé est refusé). API `GET/POST
  /api/machines/{id}/supports` (écriture réservée aux administrateurs). Au premier
  démarrage, les supports de `tarifs.json` sont repris (mode 720 × 720 en 8 passes), sans
  toucher aux prix, qui restent dans les tarifs. Compris dans la sauvegarde de la
  configuration.

*Added: media and print modes per machine with stable identifiers (`supports.json`, API
`/api/machines/{id}/supports`), migrated from the price list, included in the backup.*

## [0.13.3] — 2026-10-24

### Corrigé
- Découpe à côté de l'encre sur un PDF avec **CropBox** ou **/Rotate** : l'impression suivait
  la zone visible et le sens de la page, la coupe, l'édition, le repérage, l'imbrication et la
  rotation automatique lisaient la page brute. Chaque PDF est désormais ramené à sa zone
  visible et à son sens dès l'arrivée : une page rognée sort au format de sa zone visible, et
  la coupe tombe sur l'encre.

### Ajouté
- Tests de non-régression de la géométrie (rotation automatique avec marge et repérage, une
  coupe par lé, PDF multipage à formats et boîtes différents, échelle d'un travail tourné,
  cas où la rotation automatique doit s'abstenir) ; chacun échoue si la correction qu'il garde
  est retirée.

*Fixed: cut path off the ink on PDFs with a CropBox or /Rotate (pages are now normalised on
arrival). Added: geometry regression tests.*

## [0.13.2] — 2026-10-24

### Sécurité
- Un fichier des comptes abîmé (coupure de courant) était lu comme « aucun compte » et
  **ouvrait l'accès sans connexion**. Il garde désormais l'accès fermé.

### Corrigé
- Résistance aux coupures des fichiers d'état (file, machines, réglages, tarifs, comptes,
  travaux) : écriture synchronisée sur le disque, version précédente gardée (`.prec`) ; un
  fichier abîmé n'est plus remplacé en silence par une valeur vide (file vide, catalogue
  d'usine) : la version précédente est reprise et **Système** l'affiche.

*Security: a damaged accounts file no longer opens access without login. Fixed: state files
survive power cuts (synced writes, previous version kept and restored, shown in System).*

## [0.13.1] — 2026-10-24

### Corrigé
- Système > Média : « null » s'affichait sous les jauges tant qu'aucune consommation
  n'était relevée (un élément vide passé tel quel à l'affichage).

*Fixed: "null" shown under the ink gauges before any consumption was measured.*

## [0.13.0] — 2026-10-24

### Ajouté
- Consommation réelle de l'Epson : relevé SNMP chaque minute (encres et rouleau), gardé
  quand il change dans `releves/<machine>.jsonl` ; consommation sur 7 jours dans
  **Système > Média** et par l'API `GET /api/machines/{id}/consommation`. Seules les baisses
  comptent (un changement de poche ou de rouleau n'est pas une consommation) ; encre au %
  près.

*Added: actual Epson consumption, read over SNMP every minute (inks and roll), shown over
7 days in System > Media and available through `GET /api/machines/{id}/consommation`.*

## [0.12.1] — 2026-10-24

La 0.12.0 n'a pas été publiée (transfert de l'image de 4 Go en échec dans la CI) : la
0.12.1 la contient en entier, plus les corrections ci-dessous.

### Corrigé
- Mutoh Kona : `/dev/kona` n'était jamais créé (la règle udev mêlait des attributs de
  l'appareil USB et de l'interface dans une même condition, ce qu'udev refuse) ; constaté
  et corrigé sur le serveur de l'atelier.
- Client de bureau : **Supprimer**, la suppression d'un réglage, la restauration et
  l'enregistrement d'un réglage ne faisaient rien (le client n'affiche pas `confirm()` ni
  `prompt()`) : questions dessinées dans la page.
- Menus du client en français (Édition, Présentation, Serveur, Fenêtre), sur macOS et
  Windows.

### Modifié
- Fiche d'un travail : un grand aperçu, les couches (découpe, plaques) derrière le bouton
  **Couches**, comme dans ONYX.
- État d'un travail rendu : **Prêt** (et non « Terminé », qui laissait croire à une
  impression) ; une impression vers une machine sans pilote affiche **Déposée dans le
  dossier (pas imprimée)**.

### Modifié (publication)
- L'image d'installation est déposée directement sur la release par son job, sans
  artefact de 4 Go entre jobs.

*English: 0.12.0 was not published (4 GB image transfer failed in CI); 0.12.1 contains it
plus: Kona `/dev/kona` udev rule fixed, in-page confirmations (the desktop client shows no
confirm()/prompt(), so Delete did nothing), client menus in French, larger job preview with
layers behind a button, clearer states (Ready; left in the folder, not printed).*

## [0.12.0] — 2026-10-24

### Ajouté
- **État de l'Epson** dans Système > Média (SNMP) : longueur restante du rouleau, n° de
  support, niveaux d'encre (OR LK BK C M Y R LC LM CL), chauffages et alertes.
- **Rotation automatique** : une page trop large pour la laize de l'imprimante (repérage
  compris) qui y tient tournée de 90° est tournée seule, avec une alerte.
- **Réglages** d'un travail déjà reçu : tout le panneau de l'import (repérage, coupe, lés,
  blanc, support, couleur…), rendu refait depuis l'original.

### Corrigé
- Clé de traduction `alertes` en double (panneau Epson) qui masquait les alertes de
  pré-vérification ; contrôle des clés en double ajouté à la CI.

*English: Epson status over SNMP (roll length left, media number, ink levels, heaters,
alerts); automatic 90° rotation of a page wider than the printer's roll width; full
settings of a received job (registration, cut, tiles, white, media, colour) with a new
render from the original; fixed a duplicated translation key hiding preflight alerts.*

## [0.11.0] — 2026-10-24

### Ajouté
- Image d'installation, profils avec bureau : **CUPS** (cups, cups-browsed, pilotes
  Gutenprint, réglage des imprimantes) et la suite **ArtCraft** (13 applications d'édition)
  téléchargée pendant l'installation, empreintes vérifiées ; `sudo eko-artcraft` pour
  l'installer ou la mettre à jour ensuite.
- **Imbrication sur la forme** des contours de découpe (option du panneau Planche) : ronds
  en quinconce, triangles tête-bêche, rotations 0/90/180/270° ; l'imposition dessine les
  contours. Sur 8 ronds et 6 triangles en laize 300 mm : 202 mm de rouleau au lieu de 289.
- **Préparation du pilote Epson** : plaques → canaux K C M Y puis WH (`White`, `Blanc`,
  `WH`) ou MS (`Metallic`, `Silver`, `Argent`, `MS`) ; réglage **Blanc ou argent** sous ou
  sur la couleur, par travail ; sortie en dossier par canal avec `envoi.json`. Les tons de
  découpe ne partent jamais ; un autre ton direct, ou blanc et argent sur la même page,
  est refusé à la mise en file avec la raison.
- **Cadres Mutoh Types 2 et 3** (CutServer SF et MF) : codes POSTNET 250 × 20 mm haut et
  bas, numéro à 11 chiffres par page, cadre plein (Type 2), angles de 125 mm et
  repères intermédiaires (Type 3) ; fichier de découpe nommé `Nom_SF|MF_<numéro>.plt`.
  Type 2 éprouvé sur le Kona le 24/10 ; Type 3 à éprouver.
- **Rôles administrateur et opérateur** : l'opérateur travaille (travaux, file, planches)
  sans pouvoir régler le serveur (machines, tarifs, profils ICC, réglages enregistrés :
  refus 403 `admin_requis`, écrans masqués). `utilisateur ajouter <nom> [admin|operateur]` ;
  les comptes existants restent administrateurs.
- **Sauvegarde et restauration de la configuration** (Système, administrateur) : un
  fichier JSON avec machines, tarifs, réglages enregistrés et profils ICC ; restauration
  vérifiée en entier avant d'écrire. Sans les comptes ni les travaux.
- **Décalage de découpe** X / Y par machine (−20 à 20 mm), ajouté au HP-GL envoyé.
- **Motif step & repeat** (panneau Planche) : une page en grille colonnes × rangées, avec
  **demi-saut** en option ; découpe de chaque copie.
- **Durée d'impression estimée** sur la fiche et dans la file (total à passer par
  machine) ; cadence réglable par machine, Epson par défaut d'après sa fiche.
- **Notifications** : fin ou échec d'une sortie dit à l'écran, et au système si permis.
- **Mutoh Kona : impression + découpe éprouvée à l'atelier (24/10)** avec un cadre type 2
  du RIP : envoi comme CutServer (lecture du cadre `AL5`, coins au milieu des traits,
  `PROGSTEP10`, une commande par ligne, outil `SP1`), axes du Kona (X = hauteur de page,
  Y = −largeur), réglages de coupe par machine (vitesse, force, accélération) et par
  travail, option « découpe retournée », `.plt` prêt pour une machine (`?machine=kona`).
- **Flèche d'insertion ▲** dans les cadres Mutoh.
- Réglages de coupe comme CutServer : **valeurs du massicot**, **outil**, **motif de
  découpe** (jusqu'à 10 paires bas/haut) avec **diffusion** aux angles vifs.
- Documentation **Administrer le serveur** (FR/EN) : données, sauvegardes, mises à jour,
  comptes, machines et questions fréquentes.

### Corrigé
- Part de surface utilisée de la planche : aire des contours de découpe, et non plus celle des
  rectangles englobants.

*English: Mutoh Kona print & cut proven in the workshop with a RIP type 2 frame (frame and
barcode read, cut on target): sent as CutServer does (AL5 frame reading), Kona axes,
CutServer-like cut settings per machine (speed, force, acceleration, cutter defaults,
tool, cut pattern with spread), insertion arrow; Epson print preparation (channels,
white/silver layers, cut plates never sent); shape nesting; Kona type 2/3 frames with
POSTNET; administrator/operator roles; configuration backup and restore; cut offset;
step & repeat with half drop; print time estimate; notifications; administration guide
and FAQ; full ISO installation tested in CI; ArtCraft and CUPS on desktop profiles.*

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
  confirmé). Publiée en morceaux de 1,9 Go (limite de 2 Go des releases) avec l'empreinte
  de l'image entière.
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
