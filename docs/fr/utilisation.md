# Utilisation

On travaille dans le **client E-KO RIP Center** (voir son installation). L'écran se lit
comme une file de RIP :
- en haut, la **barre d'outils** : **Importer**, puis les actions sur le travail choisi
  (**Imprimer**, **Découper**, **Éditer**, **Réglages**, **BAT**, **Relancer**,
  **Supprimer**), la recherche, le thème (sombre ou clair, ◐) et la langue ;
- à gauche, les **vues** (Travaux, File d'attente, Système), les **machines** du
  catalogue avec le nombre de sorties en file (un clic ouvre leur file) et les
  **réglages enregistrés** (un clic ouvre l'import avec ce réglage) ;
- au centre, le **tableau des travaux** ; à droite, l'**inspecteur** du travail choisi.

Si le serveur a des comptes, le client demande d'abord de se connecter (voir *Sécurité*
dans l'installation du serveur).

## Envoyer un travail

1. **Importer** : déposer un fichier dans la zone en pointillés, ou cliquer dessus pour
   le choisir. On peut aussi lâcher un ou plusieurs fichiers n'importe où sur la
   fenêtre : ils partent tous avec les réglages de l'import.
2. Choisir le **Mode** :
   - **Imprimer (et découper)** : rendu en plaques d'encre, plus la découpe s'il y a
     des tracés de découpe ;
   - **Découper seulement** : rien n'est imprimé ; seuls les tracés partent à la
     découpeuse.
3. Régler la **Résolution** (360, 720 ou 1440 dpi), les **Tracés à découper**, le
   **Support et mode d'impression** (voir [Supports et modes
   d'impression](#supports-et-modes-dimpression)), la **Découpeuse** (voir
   [Découpe](#découpe)) et le **Repérage Kona**. Les blocs **Couleur**, **Édition
   rapide**, **Mise en file automatique**, **Blanc**, **Fond sous les repères**,
   **Tolérance de découpe** et **Mosaïque** s'ouvrent d'un clic.
4. **Envoyer au RIP**.

Formats acceptés : PDF, AI (enregistré avec « Créer un fichier compatible PDF »), SVG
(découpe seulement) et images **TIFF, PNG, JPEG**. Une image est posée à sa taille réelle,
d'après la résolution enregistrée dans le fichier (sans résolution : 72 ppi, et le travail le
signale), en CMJN, RVB ou niveaux de gris, 8 ou 16 bits, avec son profil ICC ; la
transparence ne pose pas d'encre. Taille maximale d'un fichier : 8 Go (`EKO_RIP_MAX_MO`).
Pas encore pris en charge : TIFF à palette, YCbCr, Lab ou à plans séparés, PNG entrelacé
(refusés avec un message).

**Canaux de tons directs d'un TIFF** : chacun devient sa propre plaque, posée par-dessus
les couleurs sans les effacer. Son nom est celui du canal dans Photoshop (sans nom :
`Spot1`, `Spot2`…). Un canal nommé `White`, `Blanc`, `WH` ou `Spot1` (majuscules,
espaces et `_` ignorés) donne la plaque de blanc. Les canaux de sélection de Photoshop
(masques) ne s'impriment pas. Le travail signale les plaques reprises : les regarder avant
la première impression, le sens des valeurs (canal clair = sans encre) n'a pas encore été
vérifié sur un fichier de l'atelier.

**Canal alpha → plaque blanc ou vernis** : dans la section Blanc du panneau d'envoi, le
réglage « Canal alpha » d'un PNG ou d'un TIFF détouré peut aussi donner une plaque `White`
(blanc) ou `Varnish` (vernis sélectif) qui suit exactement le détourage : opaque = pleine
encre, bord adouci compris, sans rétraction. Un alpha partout opaque donne une plaque sur
toute l'image. Si le TIFF a déjà un canal de ton direct du même nom, c'est ce canal qui
compte. Une image sans canal alpha (JPEG) n'a pas cette plaque. Le blanc généré
(sous-couche, aplat) s'y ajoute s'il est choisi aussi.

## Rotation automatique et réglages après l'import

**Tourner seul si le visuel dépasse la laize** (coché par défaut) : si la page, repérage
compris (cadre, codes, marges), est plus large que la laize de l'imprimante (Système >
Machines) mais y tient tournée de 90°, le RIP la tourne et l'indique par une alerte sur le
travail. Sans effet avec une rotation choisie dans l'édition rapide ou avec une mosaïque
(les lés s'en chargent).

**Réglages** (barre d'outils) rouvre le panneau de l'import sur le travail choisi : mode,
repérage, découpeuse, lés, blanc, support, couleur… **Appliquer et refaire le
rendu** repart de l'original, sans renvoyer le fichier.

## Édition rapide

Bloc **Édition rapide** du panneau d'envoi, ou bouton **Éditer** d'un travail fini (le
rendu est refait depuis le fichier d'origine, sans le renvoyer) :
- **Échelle** (%), **Rotation** (0, 90, 180, 270°), **Miroir (vitrophanie)** (caisson vu
  de l'arrière), **Marge blanche** autour de l'image. **Le miroir retourne** : **l'image
  et la découpe** (par défaut) ou **l'image seule** — à éprouver à l'atelier : selon la
  pose, la découpe doit suivre l'image ou non ;
- **Œillets (repères imprimés)** le long des bords cochés, à la **distance au bord**
  donnée, coins compris. **Répartition** **par espacement** (un **écart maximal**) ou
  **par quantité** (un nombre par bord). **Forme** : cercle, disque plein, croix, cercle
  et croix. Avec **Distance comptée depuis le trait de coupe (vers l'intérieur)**, le
  retrait part du contour de découpe et non du bord ;
- **Contour de découpe rectangulaire**, décalé de quelques mm (négatif = retrait vers
  l'intérieur), **autour de** l'image, du fond perdu, ou de la boîte de rognage (fond
  perdu + rogne) ;
- **Marques imprimées** (voir ci-dessous), **Fond perdu**, **Étiquette imprimée** et
  **QR code en marge**.

La découpe suit l'édition. Un travail qui a une sortie en file ne s'édite pas : annuler
ou finir la sortie d'abord. La marge s'agrandit d'elle-même pour loger les marques,
l'étiquette et le code.

**Marques imprimées** :
- **Traits de coupe aux angles**, à une **distance au fond perdu** et d'une **longueur**
  réglables ; ou **Croix de repérage aux angles** (elles remplacent les traits de coupe) ;
- **Contour imprimé sur le trait de coupe** ;
- **Repères de pli et de couture** : **Ajouter un repère**, choisir le côté et la
  **distance au côté** ; **Couture (trait continu)** ; **Imprimé** décoché, le repère
  n'est pas imprimé (20 repères au plus) ;
- **Trait** plein, pointillé ou alterné (tirets sur blanc), avec **Tiret**, **Vide**,
  **Épaisseur** et **Couleur** (noir, repérage CMJN 100 %, blanc).

Avec un repérage Kona, les traits de coupe et les croix ne sont pas imprimés (ils
gêneraient la lecture des repères) : une alerte le dit sur le travail.

**Fond perdu** sur les 4 côtés (**Côtés liés** pour la même valeur partout), jusqu'à
100 mm. **Remplissage** : une couleur (C, M, J, N en %), le premier pixel du bord, le
miroir de l'image, ou la duplication des bords. Les tons de découpe ne sont pas
recopiés dans le fond perdu. **Blanc de rogne** : blanc ajouté autour, au-delà du fond
perdu. Les marques se posent hors du fond perdu.

**Étiquette imprimée** : nom, date, L × H, lé, copies, mode et commande, en haut ou en bas
de chaque lé, au **corps du texte** choisi. Copies et référence de commande s'impriment
vides (« — ») tant que la fiche travail n'existe pas.

**QR code en marge** : en haut ou en bas, **côté du code**, **blanc autour**, niveau de
**correction d'erreur** (L, M, Q, H) et **balises** à y mettre (nom, page, tuile/total,
date, n° de travail, n° de coupe, machine, réf. commande). **Contour de découpe autour du
code** en option.

## Mise en file automatique

Bloc **Mise en file automatique** : une fois le rendu fini, chaque page imprimable part
dans la file de l'Epson et chaque page à découper dans celle du Kona (ou de la
découpeuse choisie à l'import). **Ajouter en suspendu** les met en attente : on importe
dix fichiers et on libère ceux qu'on veut depuis la file. Une découpe qui suit une
impression est toujours ajoutée suspendue (le support doit d'abord passer au Kona). Le
réglage s'enregistre avec les autres : un dossier surveillé peut ainsi imprimer seul.

On peut choisir plusieurs fichiers à la fois ; ils partent chacun avec les mêmes réglages.

## BAT (bon à tirer)

Bouton **BAT** d'un travail fini : un PDF A4 paysage à envoyer au client, une page
par page du travail : l'image en épreuve écran du profil de support, le tracé de découpe
en magenta (non imprimé), le format en mm, les réglages (mode, résolution, profil, blanc,
édition, œillets), les alertes de pré-vérification et un cadre « Bon pour accord » (date,
nom et signature). Il sort dans la langue de l'écran. Un filtre couleur est mentionné,
mais l'épreuve ne le montre pas.

**Épreuve sur une autre machine** : avec au moins deux imprimantes, la liste **BAT sur…**
de la page (à côté de **Imprimer sur…**) choisit la machine d'épreuve. **Imprimer sur**
met alors deux sorties en file : la copie **« nom-BAT »** sur la machine d'épreuve, et
l'impression demandée, **suspendue**, qui affiche **Attend le BAT** avec l'état de
l'épreuve. Une fois le BAT accepté, **Reprendre** l'original ; refusé, l'**Annuler**.
L'épreuve se fait en impression seulement, sur une autre machine que celle demandée (ni
du même pool). Le choix reste tant qu'on ne le change pas : revenir à **Sans BAT**.

## Épreuve TIFF

Le bouton **Épreuve TIFF** de la barre d'outils télécharge un TIFF de la page vue, fait
à partir des plaques rendues, c'est-à-dire de ce qui part à la machine. La page est à sa
taille exacte : l'épreuve sert aussi à vérifier les cotes d'un lé avant de l'imprimer.

Réglages, dans la section **Épreuve TIFF** du panneau d'envoi (gardés dans un réglage
enregistré) :

- **Couleur** : **Machine (CMJN)**, les valeurs d'encre telles qu'envoyées, avec le profil
  du support s'il y en a un ; ou **Colorimétrique (RVB)**, converti par le profil du support
  (intention colorimétrique relative), pour voir le rendu à l'écran.
- **Tons directs** (blanc, argent, vernis…) : **gardés en canaux** (relus comme tons
  directs par Photoshop et par le RIP), **aplatis dans la couleur** (d'après une couleur
  d'affichage : blanc en gris clair, argent en gris), ou **supprimés**. Les tons de
  découpe n'y sont jamais.
- **Taille maximale** (1 à 3500 Mo, non compressée) : au-delà, la définition baisse (un
  point pour 2 × 2, 3 × 3… points), la taille physique reste exacte.
- **Faire l'épreuve TIFF de chaque page après le rendu** : elle est prête à télécharger
  dès la fin du rendu. Sinon, le bouton la fait à la demande.
- **Épreuve seule** : l'épreuve est faite après le rendu et rien ne part à l'impression
  (ni mise en file automatique, ni bouton Imprimer ; la mise en file d'une impression est
  refusée). Le travail peut encore être découpé.

Une épreuve qui échoue n'arrête pas le travail : elle est signalée dans ses alertes.

## Éditeur visuel (visionneuse)

Un clic sur une vignette (composite, plaque, découpe) de l'inspecteur l'ouvre en grand.

**Voir.** Zoom de 25 à 800 % à la molette (le point sous le curseur reste fixe), par
**−**, **+**, la liste des %, **Ajuster** et **100 %** (taille réelle à l'écran). Outils
**Main** (glisser pour déplacer) et **Loupe** (clic : zoom avant ; Maj+clic : zoom
arrière ; tirer un rectangle : zoom sur ce rectangle). Règles en mm sur deux bords ; en
bas, les dimensions de la page et la position du curseur en mm. Clavier : `+` `−` `0` `1` et flèches.

**Laize.** Case **Laize** : la largeur du support chargé dans l'Epson (sinon la laize de
la machine) est dessinée ; ce qui dépasse est en rouge, avec l'écart en mm.

**Pages.** Un travail de plusieurs pages montre un bandeau de vignettes, une par page,
la page courante marquée. ← → ou Pg↑ Pg↓ changent de page ; la vue choisie (composite,
découpe, plaque) est gardée.

**Sélection.** Un clic sur la page la sélectionne (cadre et 8 poignées) ; un clic hors
de la page la désélectionne.

**Couches.** Panneau de droite : image, découpe (en magenta), blanc et argent,
activables une à une. Le blanc et l'argent sont les plaques aux noms que reconnaît
l'Epson, teintées sur l'image. Les repères Kona sont imprimés dans la plaque Black : pas
de couche séparée. La découpe ne se superpose que si la coupe est posée comme
l'impression (sans rotation ni miroir de [mise en coupe](#mise-en-coupe)).

### Éditer la page

Les outils d'édition ne s'ouvrent que sur un PDF rendu, sans mosaïque. Chaque changement
se prépare à l'écran ; **Appliquer** l'envoie et refait le rendu, **Annuler les
changements** revient à l'état rendu.

- **Recadrer** : poignées (tirer un bord, glisser l'intérieur pour déplacer) ou champs
  X (depuis la gauche), Y (depuis le bas), Largeur, Hauteur, en mm de la page d'origine,
  avant rotation et miroir. **Ancrage** sur 9 points, **Garder les proportions**,
  **Sans recadrage**. Un cadre qui sort de la page est refusé (au moins 1 × 1 mm).
- **Taille** : **Taille imprimée** en largeur et hauteur (mm) ou en %. Angles :
  proportionnel ; côtés : libre. **Proportions liées** décoché, largeur et hauteur se
  règlent à part, et la découpe se déforme pareil. Le coin haut gauche reste fixe.
- **Tourner de 90° à gauche** (⟲) / **à droite** (⟳), **Miroir** ; le choix **Le miroir
  retourne** (image et découpe, ou image seule) apparaît avec le miroir — à éprouver à
  l'atelier. Un travail en miroir porte le badge **Miroir** dans la file.
- Pendant un recadrage ou un changement de taille avec un repérage Kona, les repères
  sont dessinés en pointillés autour de la taille à venir.

**Annuler / rétablir** : ↶ et ↷, ou Ctrl+Z (Cmd+Z), Maj+Ctrl+Z et Ctrl+Y ; 50 états au
plus. Revenir à un état déjà rendu refait le rendu ; un état seulement saisi revient
dans le panneau.

### Filtre couleur

Bouton **Couleur** de la visionneuse : bloc **Filtre couleur**, propre au travail. Il agit
sur les plaques CMJN, après le profil de sortie et avant le blanc automatique ; blanc,
argent et découpe ne sont pas touchés.
- **Courbes** C, M, J, N : points « entrée:sortie » en % (ex. `0:0 50:60 100:100`), 16 au
  plus ; vide = aucune.
- **Luminosité** (+ = moins d'encre) et **Saturation**, de −100 à 100 %.
- **Remplacement de couleur** : une teinte CMJN **De** vers une teinte CMJN **Vers**,
  avec une **Plage** (± %), 20 au plus ; il passe avant le reste du filtre. **Ajouter un
  remplacement**, case **Actif**, **Retirer**.
- **Avant / après (gauche : avant)** : aperçu scindé, sur un filtre déjà rendu ; le
  curseur déplace la séparation. L'« après » estime l'effet sur l'épreuve écran.
- **Exporter vers le réglage** : le filtre va dans un réglage enregistré, ou dans un
  **Nouveau réglage (réglages de ce travail)** ; il est repris à l'import. **Sans
  filtre** le retire.

### Lés et tuilage

Sur une page plus large que la laize, la visionneuse montre les lés en pointillés,
leurs recouvrements teintés et leurs numéros. Panneau **Lés** :
- glisser un trait de lé (ou de rangée) donne des lés inégaux ; **Largeur des lés** et
  **Hauteur des rangées** se saisissent aussi au mm ; **Lés égaux** revient au partage
  égal ;
- **Laize**, **Hauteur maxi d'une tuile** (au-delà, la page est coupée en rangées) et
  **Recouvrement** donnent la grille, résumée en colonnes × rangées ;
- **Soudure** : le recouvrement ne reçoit pas d'encre sous la tuile voisine (12,7 mm de
  recouvrement au moins) ;
- **Trait de coupe au milieu des recouvrements** ;
- **Tuiles à imprimer** : une case par tuile ; une tuile décochée n'est ni rendue ni mise
  en file. Un clic sur son numéro règle son **recouvrement** propre (négatif = espace) ;
- **Ordre d'impression** (ex. `3 1 2`) : ces tuiles d'abord, puis les autres par numéro.

**Couper en lés** applique. Sur un travail déjà en lés, **Appliquer** garde les
changements et **Page entière (sans mosaïque)** revient à une seule page.

## Suivre les travaux

Vue **Production** (barre du haut ; **Système** à côté), en trois zones :

- **Machines** (en haut) : une ligne par machine, avec son état (prête, en cours et la
  sortie qui passe, injoignable), le support chargé, les niveaux d'encre de l'Epson, le
  nombre de sorties en file, leur poids dans le tampon (les plaques à envoyer) et la durée
  restante. Un clic n'affiche que cette machine ; un second clic les remet toutes (même
  choix que **Afficher**). Sous le tableau : le tampon de toute la file et le disque des
  données (% occupé, place libre, jauge), en rouge quand le libre passe sous le seuil de
  purge (`EKO_RIP_SEUIL_LIBRE_MO`). Ces compteurs se relisent avec la file.
- **À imprimer** : les sorties en file, dans l'ordre de la file, puis les travaux pas encore
  envoyés (« Non envoyé »). Colonnes : vignette, nom, état, machine, format, échéance,
  durée, date. Les planches en préparation s'affichent au-dessus. La colonne de boutons à
  gauche agit sur la sortie choisie : **En tête**, **Monter**, **Descendre**,
  **Suspendre**, **Reprendre**, **Annuler**, **Refaire**, **Restaurer**, **Retirer** (un
  bouton grisé ne s'applique pas à son état).
- **Imprimés** (repliable, avec leur nombre) : les sorties finies, en échec ou annulées, la
  plus récente d'abord, avec l'heure de fin et le relevé d'encre et de support.

À droite, l'**inspecteur** a quatre onglets : **Travail** (le détail ci-dessous),
**Machine** (support, encres, chauffages, alertes de la machine affichée ou de la sortie
choisie), **Réglages** (déposer un fichier sur un réglage l'importe avec ce réglage ; un
clic ouvre l'import avec lui) et **Journal** (les messages de l'écran, le plus récent
d'abord). Sur un écran de moins de 1100 px, l'inspecteur passe sous les tableaux ; sous
1360 px, les outils de la barre gardent leur symbole et leur nom passe en bulle.

**Clic droit** : sur une ligne, les actions de file de la sortie puis celles de la barre
d'outils ; sur une machine, n'afficher qu'elle, voir son état, relire l'Epson, la configurer ;
sur l'en-tête d'un tableau, les colonnes à montrer ou à masquer (nom, état et vignette
restent ; le choix est gardé par le poste). **Glisser-déposer** : dans **À imprimer**, une
sortie en attente ou suspendue se pose à la place d'une autre de la même machine.

Un clic sur une ligne ouvre son travail dans l'inspecteur :

- une partie par page, avec le nombre de tracés de découpe ;
- un bouton **Imprimer sur…** par imprimante (et par pool) et **Découper sur…** par
  découpeuse, qui mettent la page dans la file de la machine ; le badge à côté donne
  l'état de la dernière sortie ;
- **Fichier .plt** : télécharge la découpe (HP-GL, 0,010 mm par pas, origine au coin
  bas gauche de la page) ;
- l'aperçu (composite, avec l'épreuve du profil de support) ; la découpe et une vignette
  par plaque sont derrière le bouton **Couches** ;
- sur un travail à découper : les panneaux **Couches de découpe** et **Mise en coupe**,
  et sur chaque page imprimée d'un PDF, **Détourage automatique** (voir
  [Découpe](#découpe)).

**Alertes** (badge ⚠) : à l'envoi d'un PDF à imprimer, le serveur signale les
polices non incorporées (une autre sera prise au rendu : vectoriser le texte ou
incorporer la police) et les images de moins de 100 ppi à leur taille imprimée. Le
contrôle préalable de la découpe ajoute les siennes (voir [Découpe](#découpe)). Le
travail part quand même : c'est à l'opérateur de juger.

**Fiche travail** (inspecteur, repliée sous son titre, avec son résumé) : **N° de
commande**, **Client**, **Société**, **Quantité due**, **Échéance** (case à cocher, puis
la date), **Opérateur** et **Notes** ; **Enregistrer la fiche**. 200 caractères par champ,
2000 pour les notes, quantité 100 000 au plus. La quantité due est le nombre de **copies**
posées sur la planche d'imbrication et s'imprime sur l'étiquette avec le n° de commande ;
l'**échéance** est la priorité de l'imbrication (la plus proche passe d'abord, sans
échéance ensuite, puis l'ordre d'arrivée). Le tableau a une colonne **Échéance**, et le
choix **Tri** (**Arrivée** ou **Échéance**) au-dessus.

**Imprimer cette page** (case sur chaque page de l'inspecteur, cochée par défaut) :
décochée, la page n'entre ni dans la file ni dans une planche d'imbrication (badge **Non
imprimée**) ; une sortie déjà en file attend avec la raison **Page exclue**. Recochée, elle
revient. Une page décochée ne se met pas en file à la main non plus.

La barre de recherche filtre par nom, n° de commande, client ou société. **Relancer** refait un travail en échec ;
**Supprimer** efface un travail fini (refusé tant qu'une sortie l'attend dans la file).

## File d'attente

La file de chaque machine se lit dans la vue **Production** (zone **À imprimer**, filtrée
par machine). Le nombre de sorties à passer s'affiche sur le bouton **Production**.

- **Mutoh Kona 1650** : la découpe part par le port du plotter (`/dev/kona`).
- **Epson SC-S80600** : en connexion **Réseau (TCP)**, port 9100, le RIP imprime par son
  propre pilote (voir [Pilote direct de l'Epson](#pilote-direct-de-lepson)). En
  connexion **Dossier**, rien n'est imprimé : les plaques sont copiées dans
  `/var/lib/eko-rip/sorties/epson/<travail>-p<page>-<sortie>/` sur le serveur, un
  fichier par canal de l'imprimante (`K.tif`, `C.tif`, `M.tif`, `Y.tif`, puis `WH.tif`
  ou `MS.tif`) et `envoi.json` (canaux dans l'ordre, plaque d'origine, couches) ; la
  sortie finie s'affiche **Déposée dans le dossier (pas imprimée)**.

Les tons de découpe (`CutContour`…) ne partent jamais à l'imprimante. Le blanc est
reconnu sous les noms `White`, `Blanc`, `WH`, l'argent sous `Metallic`, `Silver`,
`Argent`, `MS` (Epson conseille `White` et `Metallic`). **Blanc ou argent** (section
Blanc du travail) le pose sous la couleur ou dessus (vitrophanie). La mise en file est
refusée, avec la raison, pour un autre ton direct (le convertir en CMJN ou le renommer),
ou pour du blanc et de l'argent sur la même page.

Chaque impression à passer affiche sa **durée estimée**, et l'en-tête de la colonne le
total restant ; la fiche du travail aussi. Estimation : longueur imprimée × laize de la
machine ÷ cadence. Cadence de l'Epson par défaut (fiche Epson) : 94 m²/h jusqu'à 360 dpi,
17 m²/h à 720 dpi (720 × 1440, 6 passes), 6,6 m²/h au-delà ; **Cadence (m²/h)** dans
l'éditeur des machines la remplace (à mesurer à l'atelier, et pour la Roland). Quand une
sortie se termine ou échoue, l'écran le dit, et le système aussi si le client a
l'autorisation d'afficher des notifications.

Chaque machine passe une sortie à la fois, dans l'ordre de sa colonne. Sur une sortie
à passer : **En tête**, **Monter**, **Descendre**, **Suspendre** / **Reprendre**,
**Annuler**. Une sortie suspendue ne part pas ; les suivantes passent devant elle.
L'**Historique** garde les sorties finies : **Refaire** relance la même sortie sans
refaire le rendu, **Retirer** l'efface de l'historique. L'**Archive** garde les découpes
archivées (voir [Après la coupe](#couper-dès-larrivée-et-après-la-coupe)) : **Restaurer**
ou **Retirer**.

**Raison d'attente.** Une sortie qui ne peut pas partir le dit par un badge rouge (le
survol donne le détail), et laisse passer les suivantes (une sortie suspendue par un
opérateur reste simplement *Suspendue*) :
- **Machine injoignable** : machine débranchée ou absente du catalogue ; la sortie part
  dès qu'elle répond ;
- **Fichier manquant** : rendu ou tracé introuvable ; relancer le travail ;
- **Trop large** : la page, repères compris, dépasse la laize de la machine ou du rouleau
  chargé ;
- **Autre support** : le support chargé n'est pas celui du travail ; la sortie part
  d'elle-même quand le bon est chargé ;
- **Impression pas finie** : une découpe attend que l'impression de sa page soit finie ;
- **Découpeuse indisponible** : une impression attend que la découpeuse affectée soit
  utilisable (pas d'encre gâchée) ;
- **En imbrication** : l'impression attend sa planche (voir [Imbrication dans la
  file](#imbrication-dans-la-file)) ;
- **Page exclue** : la page est décochée (**Imprimer cette page**).

Une impression posée sur une planche passe **Imbriquée** (« sur » le nom de la planche) : elle ne
part plus seule, la planche l'imprime. Elle ne se **Refait** pas tant que sa planche n'est
pas imprimée (ce serait l'imprimer deux fois) ; **Retirer** reste possible.

Une sortie coupée par un arrêt du serveur ne repart pas seule (une découpe ne se
reprend pas au milieu) : elle passe en *Échec*, et **Refaire** la relance.

## Pilote direct de l'Epson

Avec une Epson SC-S80600 en connexion **Réseau (TCP)**, **Adresse IP** de l'imprimante et
**Port TCP** 9100 (Système > Machines), le RIP envoie lui-même l'impression : les plaques
C, M, J, N du rendu sont séparées sur les 9 encres de la machine (couples clair et foncé,
limite d'encre), tramées en 4 tailles de point et envoyées bande par bande. Premier aplat
CMJN imprimé le 25/10, couleurs justes.

- **Alertes de l'imprimante** : avec une adresse **SNMP** sur la machine, l'état est lu
  avant l'envoi, puis toutes les 5 s jusqu'à la fin de l'impression. Une alerte
  bloquante (capot ouvert…) arrête l'envoi : la sortie passe en *Échec* avec « L'imprimante
  signale une alerte : impression arrêtée ». La chauffe compte comme impression en cours.
  Sans SNMP, la sortie est finie dès que tout est envoyé.
- **Tête vide** : l'image commence 20 mm après le début de la page, pour éviter la bande
  foncée vue en tête au premier tirage.
- **Plafond d'encre** : chaque canal est limité à 85 %, contre les fines bandes vues dans
  le jaune au premier tirage. Tête vide et plafond sont à confirmer au second tirage.
- **Origine X / Y impression (mm)** de la machine : place l'image sur la page envoyée,
  X depuis le bord de référence de la laize (arrondi à 16 points), Y ajouté à la tête vide.
  0 par défaut, à régler sur un essai papier.
- **Blanc et argent** ne passent pas encore par le pilote direct : la mise en file
  vers une Epson en Réseau est refusée (« Envoi à l'imprimante impossible ») ; passer
  par une machine en **Dossier** en attendant.

Limites connues : seuls les tirages du 25/10 ont été faits ; les autres résolutions et
les supports réels restent à éprouver à l'atelier.

## Les plaques

Une plaque par encre : Cyan, Magenta, Jaune, Noir, plus une plaque par ton direct
du PDF. Pour l'Epson SC-S80600 :

- **Blanc** : nommer le ton direct `White` dans Illustrator ;
- **Argent métallisé** : nommer le ton direct `Silver`.

Sur l'aperçu d'une plaque, le noir est l'encre et le blanc l'absence d'encre.

Les tons de découpe (`CutContour`…) ne sont jamais imprimés et n'effacent pas l'encre
sous eux, même sans surimpression dans le fichier : au rendu, ils deviennent le colorant
`None` de la norme PDF. Il n'y a donc pas de liseré blanc le long de la découpe.

## Blanc automatique

Bloc **Blanc** (mode Imprimer) : la plaque `White` est générée après le rendu.
- **Sous-couche sous l'encre** : blanc partout où une encre dépasse l'**encre minimale**
  (5 % par défaut) — pour imprimer sur support transparent ou coloré.
- **Aplat sur toute la page** : blanc sur toute la page.
- **Rétraction** (0,2 mm par défaut, jusqu'à 5 mm) : le blanc recule d'autant depuis le
  bord de l'encre (et du bord de la page), pour ne pas dépasser de la couleur.

Le blanc déjà dessiné dans le fichier (ton direct `White`) est gardé : le blanc final
réunit les deux. La plaque générée apparaît dans les aperçus, compte dans le coût et part
à l'Epson avec les autres plaques (en connexion Dossier pour l'instant, voir le pilote
direct). Un réglage enregistré garde le choix de blanc.

## Découpe

Les tracés peints dans un ton de découpe sont repérés. Un ton à 0 % n'est pas découpé.
Avec **Tous les tracés**, chaque tracé du fichier est découpé (fichiers de découpe seule).
Les tracés sont rognés à la page, et au cadre d'un formulaire du PDF : la lame ne sort
jamais du support.

### Préfixes de découpe

**Système > Découpe**, **Préfixes de découpe (séparés par ;)** : un ton est de découpe si
son nom **commence** par un préfixe. Par défaut : `CutContour`, `Thrucut`, `Kisscut`,
`InsideCut`, `Perforation`, `Creasing`, `Score`, `VCut`. Majuscules, espaces et tirets
indifférents : `CutContour` reconnaît `CutContour1` et `CutContourKissCut`, pas
`1CutContour`. Un changement vaut pour les rendus suivants : relancer un travail déjà
rendu.

### Couches de découpe

Panneau **Couches de découpe** de l'inspecteur : **une couche par nom de ton**, avec son
nombre de tracés.
- **L'ordre des couches est l'ordre de coupe** (↑ / ↓) : mi-chair avant coupe à travers.
  Ordre de départ : rainage (`Creasing`, `Score`), perforation et V-cut d'abord, la coupe
  en dernier, quand la matière tient encore.
- La case de la couche la coupe ou non.
- **Réglage de coupe** : par défaut, **— même nom, sinon défaut —** : le réglage de la
  découpeuse qui porte le nom de la couche s'applique seul, sinon le premier de la
  machine. On peut en imposer un autre.
- Rien n'est gardé sans **Enregistrer les couches** (les fichiers de coupe sont alors
  refaits) ; **Annuler** revient aux couches enregistrées. Les couches enregistrées
  restent au redémarrage et au rendu suivant.

Un travail d'avant les couches garde son ancien réglage de coupe : il va à la couche du
même nom ou à la seule couche ; sinon le panneau le signale, à choisir couche par couche.

Dans une couche, un contour contenu dans un autre (le trou d'un « O », une fenêtre) est
découpé avant lui, pour que la pièce ne bouge pas ; à profondeur égale, la lame va au
contour le plus proche (voir le tri des tracés plus bas).

Sur la machine, **Une couche à la fois** (Système > Machines) : pour une découpeuse qui ne
change ni d'outil ni de réglage en cours de coupe, chaque couche devient une sortie, la
suivante retenue jusqu'à **Reprendre**.

### Contrôle préalable

Au rendu, le RIP signale (alertes du travail) :
- un ton de découpe défini en RVB (le définir en CMJN, ton direct) ;
- un tracé de découpe avec un fond (un tracé de découpe est un filet seul) ;
- un tracé qui touche le bord de la page ;
- un tracé recouvert par l'image (le mettre sur le calque du dessus) ;
- un contour vectorisé (filet devenu surface : il serait coupé deux fois) ;
- un filet aligné à l'intérieur ou à l'extérieur du tracé (l'aligner au centre) ;
- un tracé qui sort du plan de travail (ce qui dépasse n'est pas coupé) ;
- une découpeuse choisie sans aucun ton de découpe reconnu (les tons du fichier sont
  listés) ;
- un tracé sur un calque non imprimable : il n'est pas coupé, sauf avec **Couper aussi
  les tracés des calques non imprimables** (Système > Découpe) ;
- des tracés ouverts non coupés, si les tracés ouverts ne sont pas autorisés.

### Découpeuse et coupe après impression

**Découpeuse** (panneau d'envoi) : la machine où va la découpe automatique. Tant
qu'elle n'est pas utilisable (absente, hors ligne, mal connectée, ou une couche sans
réglage sur elle), l'impression attend : rien ne part, pour ne pas gâcher d'encre ni de
support. Une découpe attend que l'impression de sa page soit finie à 100 %.

### Mise en coupe

Panneau **Mise en coupe** de l'inspecteur. Il tourne le travail pour la découpeuse, pas
le fichier, sans nouveau rendu :
- **Rotation de la coupe** par quart de tour, cadre de repérage compris : tourner le
  support d'autant. L'aperçu de coupe porte une flèche grise du sens d'avance (le haut de
  la page entre le premier) ;
- **Décaler la coupe vers l'extérieur (mm)**, de 0 à 5 mm : la pièce grandit d'autant
  (un trou se resserre) ;
- **Compensation de cintrage** (travail imprimé) : voir plus bas ;
- en **découpe seule** seulement (sans impression, la coupe n'a pas d'encre à suivre) :
  **Miroir**, **Échelle X / Y (%)** (**Proportionnelle** ou non), **Copies X / Y** avec
  **Écart X / Y (mm)** (100 par sens, 1000 en tout), **Départ depuis l'origine de la
  machine**.

**Enregistrer la mise en coupe** refait les fichiers de coupe. Le décalage ne remplace
pas l'étalonnage imprimante-découpeuse (décalage X / Y de la machine).

### Tolérance de découpe

Bloc **Tolérance de découpe** du panneau d'envoi, **Étendre l'image près des tracés** :
l'encre qui s'arrête à moins du **seuil de proximité** d'un contour fermé est prolongée
jusqu'au contour, puis au-delà sur la **largeur** donnée (0,1 à 10 mm) : la coupe ne
laisse pas de liseré blanc. L'aperçu composite ne le montre pas, les plaques si. Valeurs
prudentes ; ne remplace pas l'étalonnage.

### Tracés ouverts et tri

Dans **Système > Découpe** (actifs par défaut) :
- **Autoriser les tracés ouverts (évents de bâche)** : décoché, un tracé ouvert n'est pas
  coupé, avec une alerte par page ;
- **Trier les tracés** : intérieurs d'abord, puis le plus proche voisin ; décoché, les
  tracés sont coupés dans leur ordre de création.

### Couper dès l'arrivée et après la coupe

**Couper dès l'arrivée** (machine, Système > Machines, coché par défaut) : décoché, chaque
découpe arrive suspendue dans la file de la machine et part quand l'opérateur clique
**Reprendre**.

**Après coupe** (Système > Découpe) : **ne rien faire (historique)** (par défaut),
**archiver (restaurable)**, ou **supprimer de la file**. Une découpe archivée sort de la
file et va dans l'**Archive** de sa machine ; **Restaurer** la remet en file (suspendue
sur une machine sans coupe à l'arrivée).

### Détourage automatique

Panneau **Détourage automatique** de chaque page imprimée d'un PDF : le RIP analyse
l'image rendue, retire le blanc autour et propose un contour de découpe (`CutContour`).
Réglages : **Seuil du blanc (1–255)**, **Supprimer les plus petits que (mm²)**,
**Lissage (0–5)**, **Décalage vers l'extérieur (mm)**, **Supprimer les contours
intérieurs**. **Aperçu** d'abord (nombre de contours et dessin), puis **Accepter** : les
fichiers de coupe sont refaits. **Retirer le détourage** l'ôte. Un nouveau rendu
l'efface. Seulement sur le rendu d'un PDF.

### Compensation de cintrage

Dans **Mise en coupe**, **Compenser le cintrage** : une fois les sécheurs chauds,
l'impression sort courbée (milieu en retard sur les bords) ; la coupe suit. Régler la
**Largeur du support (mm)** (la courbe y est centrée), le **Début (mm d'avance)**, compté
depuis le haut de la page (rien avant), et la **Flèche (mm)**. La courbe s'installe sur
50 mm. À mesurer sur un tirage.

### Variantes de machine

Système > Machines, **Dupliquer (autre jeu de repères)** sur une découpeuse : une
variante, même machine physique, avec son propre **Jeu de repères** (ou **— celui du
travail —**). Connexion, modèle et réglages de coupe sont repris de la machine d'origine
à chaque enregistrement. Un travail affecté à la variante prend ses repères ; sa
découpe passe par la machine d'origine, une sortie à la fois.

## Repérage pour le Kona

En mode **Imprimer**, le choix **Repérage Kona** ajoute autour de l'image :
- **Croix P1–P4** : quatre croix aux coins, à 10 mm de l'image ; sur le Kona,
  Actions > Contour Cut > Manual, pointer P1, P2, P3, P4.
- **Cadre Mutoh (type 1)** : cadre noir de 5 mm, à 10 mm de l'image, intérieur d'au
  moins 250 × 250 mm, blancs de 25 mm sur les côtés, 30 mm à l'avant, 80 mm à
  l'arrière (cotes minimales du manuel Kona respectées). ⚠️ Le Kona ne lit pas seul un
  cadre sans code-barres (manuel, § 6.2 : manuel, Single Frame, Multi Frame) : envoyé
  directement, il s'arrête sur « EPOS 6213 ». Pour la lecture automatique, prendre le type 2.
- **Cadre Mutoh + code-barres (type 2, CutServer SF)** : **éprouvé à l'atelier le 24/10**
  (impression, lecture du cadre et des codes, coupe). Le même cadre et deux codes POSTNET de 250 × 20 mm (bas à gauche, haut à droite,
  à 50 mm du côté et 9 mm du cadre) avec le numéro en clair. Le numéro (11 chiffres)
  vient du travail et de la page. Le fichier de découpe téléchargé porte le nom attendu
  par Mutoh CutServer, `Nom_SF_<numéro>.plt` : le déposer dans son dossier, le Kona lit
  le code et CutServer envoie le fichier du même numéro. Intérieur d'au moins 310 mm de
  large.
- **Cadre multi-segments + code-barres (type 3, CutServer MF)** : angles de 125 mm, un
  repère de 30 mm tous les 500 mm au plus sur chaque côté, mêmes codes, fichier
  `Nom_MF_<numéro>.plt`, pour les grandes longueurs découpées par segments.

**Multi-cadre automatique** : avec le type 2, une page de **1,5 m et plus** dans le sens
d'avance prend d'elle-même le cadre multi-segments (type 3), et son fichier le suffixe
`MF`. Le manuel Kona (§ 6.3) déconseille le cadre simple au-delà de 2 m et le
multi-cadre sous 1,5 m.

Types 2 et 3 suivent les tableaux du guide d'application Kona (chap. 4.4.2) ; la place
du code du haut et l'écart des repères du type 3 sont lus sur ses schémas, sans cote (le
type 3 reste à éprouver).

Chaque cadre porte une **flèche ▲** dans son coin intérieur bas-droit : elle pointe vers
le bord qui sort le premier de l'Epson, celui qu'on engage le premier dans le Kona (par
l'arrière).

**Alertes de repérage** : une image de moins de 25 cm de côté est signalée (taille
conseillée pour le repérage Kona). L'alerte « moins de 5 cm de blanc » entre l'image et
les repères est suspendue : nos cadres laissent 10 mm, la règle reste à éprouver sur la
machine.

**Fond sous les repères** (bloc du panneau d'envoi), pour un support sombre, transparent
ou réfléchissant : **Imprimer un fond sous les repères** pose une bande d'encre qui
déborde de chaque repère et code-barres (**Débord autour des repères**, 0,5 à 13 mm), ou,
avec **Aplat de toute la zone des repères (hors image)**, toute leur zone. **Encre** :
blanc (ton `White`) ou CMJN (valeurs en %). Les repères noirs sont imprimés dessus en
surimpression : le fond reste aussi sous eux.

**Envoi au Kona.** Le RIP envoie la découpe comme Mutoh CutServer : avec un cadre, il
déclare les deux coins opposés du cadre (milieux des traits), le Kona le balaie, lit le
code, puis coupe. Les **réglages de coupe** (vitesse en cm/s, force en g, accélération en
G) se déclarent par machine dans Système > Machines, comme les « couches de découpe » de
CutServer ; le premier sert par défaut. Comme dans CutServer, chacun peut aussi garder les
**valeurs du massicot** (rien n'est envoyé), imposer un **outil** (1 à 4), et suivre un
**motif** de découpe : jusqu'à 10 paires « bas/haut » en mm (`8/2 8/2` = couper 8 mm,
lever 2 mm), avec la **diffusion** (un nombre entier de motifs par ligne droite, la coupe
commence et finit aux angles vifs). Chaque couche de découpe part avec son propre réglage.
**Fichier .plt** avec `?machine=kona` donne le
fichier tel que le Kona le reçoit, à envoyer à la main (port série 9600, 8N1, RTS/CTS).
Si la coupe sort tournée de 180°, cocher **découpe retournée** sur la machine.

La page imprimée est agrandie d'autant, et le fichier de découpe suit le même décalage.
Le Kona doit être réglé sur une résolution de **0,010 mm**.

## Réglages et dossiers surveillés

**Enregistrer…** garde tout le panneau sous un nom (lettres, chiffres, espaces, `-`,
`_`, `+`). Le choisir dans **Réglage** remplit le panneau ; **Supprimer** le retire.

Chaque réglage a un **dossier surveillé** : `/var/lib/eko-rip/entrees/<nom du réglage>/`
sur le serveur. Un PDF, AI, SVG, TIFF, PNG ou JPEG déposé dedans devient un travail avec ce réglage, dès
que sa copie est terminée (taille stable pendant 5 s). Un fichier refusé est rangé dans
`refuses/`, avec un fichier `.refus.txt` qui donne la raison.

**Dossier automatique** : `/var/lib/eko-rip/entrees-auto/` choisit le réglage lui-même,
d'après les **Règles du dossier automatique** (administrateur, Système). Elles sont
essayées de haut en bas (**↑** pour remonter une règle), et la première qui correspond
donne son réglage :

- **Motif du nom** : `*` pour n'importe quels caractères, `?` pour un seul, sans tenir
  compte des majuscules. `BACHE_*` prend `BACHE_client-12.pdf`, pas `affiche_BACHE.pdf`.
- **Petit côté** et **grand côté** (mm, facultatifs) : le format de la première page d'un
  PDF, quelle que soit son orientation. Une borne vide vaut 0 ou 100 m. Une règle de format
  ne prend jamais un fichier dont le format n'est pas lu (image, SVG, PDF de plus de
  512 Mo) : mettre une règle sans format après elle.

Exemple : `BACHE_*` → Bâche ; `*.pdf`, grand côté de 2000 à vide → Grand format ; `*` →
Vinyle. Un fichier qu'aucune règle ne prend va dans `entrees-auto/refuses/` avec sa raison.
Les règles sont dans la sauvegarde de la configuration.

**Plusieurs rendus à la fois** : le serveur rend plusieurs travaux en même temps (un par
cœur, 4 au plus, réglable par `EKO_RIP_RENDUS`), montrés dans Système > **Rendus
simultanés**. Les arrivées des dossiers surveillés et de l'ERP partent en file dans leur
ordre de dépôt, même quand un petit fichier finit son rendu avant un gros arrivé plus tôt.
Un fichier envoyé à la main depuis l'écran passe devant les arrivées automatiques qui
attendent leur rendu, et part dès qu'il est prêt.

**Motif à balises du nom** (même éditeur, facultatif) : le nom du fichier porte lui-même le
réglage et le reste. Avec `{nom}__{machine}__{reglage}__{copies}`,
`Affiche__epson__bache__30.pdf` donne le réglage Bâche, l'Epson et 30 copies. Chaque balise
prend le texte jusqu'au séparateur qui la suit ; la dernière va jusqu'à l'extension. Le
séparateur double `__` est conseillé : un `_` seul se trouve souvent dans les noms. Deux
balises doivent être séparées, et chacune ne sert qu'une fois.

| Balise | Effet |
|---|---|
| `{nom}` | Texte libre, sans effet. |
| `{reglage}` | Réglage enregistré, sans tenir compte des majuscules, accents, espaces, `-` ni `_` (`bache` = « Bâche »). |
| `{machine}` | Machine du catalogue, par identifiant ou par nom : imprimante de la mise en file automatique, ou découpeuse du travail. |
| `{support}` | Support de l'Epson, par nom. |
| `{copies}` | Quantité de la fiche travail (1 à 100 000). |
| `{repetition}` | Nombre d'impressions de chaque page mises en file d'office (1 à 100). |
| `{laize}` | Laize de la mosaïque du réglage, en mm (`1600` ou `1600mm`). |
| `{echelle}` | Échelle en % (`200`, `12,5%`). |
| `{largeur}` / `{hauteur}` | Taille voulue en mm, pour un PDF : l'échelle en découle (la plus petite des deux si les deux sont données). |

Un nom qui ne suit pas le motif, ou une valeur qui ne se lit pas, n'est pas refusé : le
travail passe par les règles et porte une alerte (« Nom hors du motif à balises », « Balise du nom non reconnue »),
aussi écrite dans le journal du serveur. Sans `{reglage}` reconnu, ce sont les règles qui
choisissent le réglage ; si aucune ne prend le fichier, la raison du refus donne ces alertes.

Deux cases du panneau d'envoi, gardées dans le réglage, servent surtout aux dossiers
surveillés :

- **Une page par travail** : un PDF de plusieurs pages devient un travail par page, nommés
  « fichier.pdf (p. 1/5) », « fichier.pdf (p. 2/5) »… avec les mêmes réglages. Sans effet
  sur une planche imbriquée.
- **Refuser si une police manque** : le fichier est refusé (rangé dans `refuses/` pour un
  dossier surveillé) dès qu'une police n'est pas incorporée, avec la liste des polices.
  Les 14 polices de base du PDF (Helvetica, Times, Courier, Symbol, ZapfDingbats) ne
  comptent pas : le rendu les remplace par des polices de mêmes dimensions. Sans la case,
  le fichier passe avec une alerte.

**Exporter, importer** (administrateur, Système > Sauvegarde de la configuration) :
**Exporter les réglages** télécharge tous les réglages enregistrés, pour un autre serveur ;
**Importer des réglages** reprend ceux d'un export. **Si le nom existe** : **renommer
(« nom 2 »)** ou **écraser**. Tout est contrôlé avant d'écrire.

## Mosaïque

Bloc **Mosaïque** : cocher **Couper en lés si plus large que la laize**, régler la laize
et le **recouvrement** (zone imprimée sur les deux lés voisins, pour coller ou souder).
Les lés ont d'abord la même largeur. Chaque lé devient une page du travail, avec sa
propre découpe rognée à son bord. **Numéro et traits d'assemblage** ajoute en pied de
chaque lé une bande de 10 mm : « Lé 2/3 » et un trait vertical là où le lé voisin
s'arrête ou commence, pour aligner les lés à la pose.

Le bloc règle aussi la **Hauteur maxi d'une tuile** (la page est alors coupée en
rangées : colonnes × rangées), la **Soudure** (recouvrement sans encre sous la voisine,
12,7 mm au moins) et le **Trait de coupe au milieu des recouvrements**. Lés inégaux,
tuiles à ne pas imprimer, recouvrement propre à une tuile et ordre d'impression se
règlent dans la visionneuse (voir [Lés et tuilage](#lés-et-tuilage)).

## Planche (imbrication)

Sur chaque page imprimable, cocher **Ajouter à la planche** et indiquer le nombre de
copies. Le panneau **Planche** apparaît au-dessus de la liste : régler la **laize**
(largeur utile du rouleau, 1600 mm par défaut) et l'**espacement**, puis **Créer la
planche**. Avant de la créer, le panneau dessine l'**imposition** : le rouleau à la laize,
chaque élément à sa place (survol : nom et dimensions), la longueur de rouleau et la part
de surface utilisée ; il se met à jour à chaque changement. Avec **Tourner de 90° si la planche est plus courte**, quatre orientations
sont essayées (telle quelle, tout tourné, tout couché, tout debout) et la plus courte en
rouleau est gardée ; un élément plus large que la laize n'y passe que tourné. La
résolution et le repérage du panneau Nouveau travail s'appliquent. Les
éléments sont rangés du plus haut au plus bas, de gauche à droite ; la planche est un
nouveau travail, avec la découpe de tous les contours.

**Contour de toute la planche (couche NestTrimBox)** ajoute un rectangle de découpe autour
des éléments posés, dans sa propre couche `NestTrimBox`, coupée en dernier et jamais
imprimée. **Marge haute (mm)** et **Marge basse (mm)** (0 à 200) laissent du blanc
au-dessus et au-dessous des éléments, pour la table de découpe.

Une page qui porte des repères de découpe (repérage Kona, cadre…) sans aucun tracé de
coupe reçoit l'alerte : *Des repères de découpe sont imprimés, mais la page n'a aucun
tracé de coupe : encre et support pour rien.*

Avec **Imbriquer sur la forme des contours de découpe**, chaque élément occupe la place de
son contour de découpe, plus l'espacement, au lieu de son rectangle : ronds en quinconce,
triangles tête-bêche (avec la rotation, les angles 0, 90, 180 et 270° sont essayés). Un
élément sans contour fermé garde son rectangle. L'imposition dessine alors les contours.
Sur une grille de pas égal au 1/800 de la laize (1 mm au moins) : une découpe ne touche
jamais sa voisine, mais l'espace réel peut dépasser l'espacement demandé d'un pas. La part
de surface utilisée compte l'aire des contours de découpe, dans les deux modes.

**Motif step & repeat** répète une seule page en grille : **colonnes** × **rangées**, à
l'espacement choisi. Avec **Demi-saut**, une colonne sur deux descend d'une demi-hauteur
(raccord papier peint). Le nombre de copies coché est ignoré ; la grille doit tenir dans
la laize (100 colonnes, 500 rangées et 2000 copies au plus). La découpe suit chaque copie.

### Planche manuelle (API, écran à venir dans les clients natifs)

`POST /api/planches` et `/api/planches/apercu` acceptent trois champs de plus (un seul
parmi `motif`, `manuel` et `gabarit`) :

- `manuel` : `{ "poses": [{ "copie": 0, "x_mm": 0, "y_mm": 0, "angle": 0, "verrou": true }],
  "grille_mm": 10, "magnetisme_mm": 5 }`. Les copies sont comptées dans l'ordre des
  éléments, à partir de 0 ; x et y donnent le coin bas gauche sur la planche. Une copie
  **verrouillée** est prise telle quelle. Les autres sont calées sur la **grille**, puis
  **aimantées** (à moins de `magnetisme_mm`) au bord de la laize ou au bord d'une copie déjà
  posée, à l'espacement près. Une copie qui sort de la laize ou en chevauche une autre est
  refusée (`planche_invalide`, avec le numéro). Les copies sans pose vont en rangées au-dessus.
- `gabarit` : `{ "nom", "largeur_mm", "hauteur_mm", "colonnes", "rangees", "espace_mm" }`.
  Une copie par case, de gauche à droite puis de bas en haut, centrée et couchée si elle
  ne tient que tournée. Les gabarits se gardent par nom avec `GET`/`POST /api/gabarits`
  (administrateur) et passent dans la sauvegarde de la configuration.
- `donnees_variables` : `{ "csv": "Nom;Numéro\n…", "champs": [{ "colonne": "Nom",
  "x_mm": 10, "y_mm": 5, "taille_pt": 12 }] }`, sur une seule page. Il y a une copie par
  ligne du CSV (2000 au plus ; séparateur `;` ou `,` pris dans l'en-tête ; guillemets
  doubles). Chaque champ écrit la valeur de sa colonne en Helvetica noir, à x, y mm du coin
  bas gauche de la page. Les données variables se combinent avec `gabarit` ou `manuel`.

## Imbrication dans la file

Au lieu de sortir une à une, les impressions en attente d'une même imprimante peuvent
partir ensemble sur une **planche** : elles sont posées côte à côte sur la laize, et la
planche part quand son **déclencheur** est atteint, ou sur **Imprimer maintenant**.

### Réglage (Système > Machines)

Pour chaque imprimante, sous ses réglages (le survol des lignes donne l'aide) :

- **Imbrication** (stratégie) :
  - **Un par un** (défaut) : chaque impression sort seule, rien n'est imbriqué ;
  - **Groupé dans l'ordre** : côte à côte, par priorité (échéance la plus proche
    d'abord), sans rotation ni réordonnancement ;
  - **Économie de support** : toute la planche est recalculée à chaque arrivée, les plus
    hauts d'abord, avec le quart de tour quand il raccourcit la planche.
- **Déclencheurs** (vides : pas de déclencheur ; plusieurs : le premier atteint) :
  - **Départ après (min)** : minutes depuis l'arrivée du **premier** travail de la
    planche ;
  - **Départ sans arrivée depuis (min)** : minutes sans **nouveau** travail (comptées
    depuis le dernier arrivé) ;
  - **Départ à (m²)** : surface des travaux posés, copies comprises ;
  - **Longueur cible (mm)** : les travaux qui la feraient dépasser attendent la planche
    suivante (badge *attend la planche suivante*), et une planche pleine part d'elle-même ;
    un travail seul plus long que la cible part quand même, seul ;
  - **Départ à (% de la cible)** : la planche part quand elle atteint ce pourcentage de
    la longueur cible (demande une longueur cible) ;
  - **Départ à la longueur (mm)** : la planche part dès qu'elle atteint cette longueur ;
    rien n'est reporté (le travail qui la franchit part avec elle) ;
  - **Rangée pleine sortie seule** : le travail qui ouvrirait une deuxième rangée attend,
    et la rangée pleine part seule.

  Sans aucun déclencheur, la planche attend **Imprimer maintenant**.
- **Chargement** : **Libre (laize du rouleau)**, la planche prend la laize du rouleau
  chargé (sinon celle de la machine), quel que soit le format des travaux ; **Strict (une
  planche par format)**, les travaux de formats différents ne partagent pas une planche, et
  chacune est posée sur son format (borné par le rouleau). Le format d'un travail est la
  laize de son support au catalogue, sinon celle pour laquelle sa mosaïque est faite ; sans
  format connu, le rouleau.
- **Espace entre copies (mm)** (5 par défaut) : blanc entre deux éléments côte à côte,
  et entre rangées ; **Espace entre rangées (mm)** le remplace entre rangées (vide : le
  même).
- **Justification** : **Gauche**, **Centre** ou **Droite** : place du groupe d'éléments
  sur la laize. Centré ou à droite, la planche fait toute la laize.
- **Décalage d'origine X (mm)** (0 à 1000) : blanc à gauche de la planche, avant tout le
  reste.
- **Bande de purge** : **Aucune**, **À gauche**, **À droite** ou **Des deux côtés** ;
  **Largeur de purge (mm)** (1 à 100, 10 par défaut). La bande court le long de toute la
  planche, au bord choisi (à gauche, après le décalage d'origine), en quatre bandes égales
  cyan, magenta, jaune, noir, pour dégorger chaque tête ; l'espace entre copies la sépare
  des éléments.
- **Retourner un lé sur deux** : les lés pairs d'une mosaïque (2, 4…) sont posés à
  l'envers (demi-tour, même place), pour un raccord de teinte entre lés voisins.
- **Marge haute (mm)** et **Marge basse (mm)** (0 à 200) : blanc au-dessus et au-dessous
  des éléments, pour la table de découpe.
- **Contour de toute la planche (couche NestTrimBox)** : rectangle de découpe autour des
  éléments posés (sans le décalage ni la purge), dans la couche `NestTrimBox`, coupée en
  dernier, jamais imprimée.

### Ce qui s'imbrique ensemble

Seuls partagent une planche les travaux qui ont le même **support**, le même **mode
d'impression**, la même **découpeuse** (« aucune » comprise : un travail à couper au Kona
et un travail sans découpe font deux planches) et le même **miroir** ; et ce que la planche
n'a qu'une fois : la **résolution**, le **repérage**, la **couleur** (profils, intention),
le **blanc**, les couches d'encre, le **Fond sous les repères**, les **Tracés à
découper** et la **Tolérance de découpe** ; enfin les **réglages de coupe choisis par couche** (deux travaux qui
choisissent deux réglages différents pour une même couche ne s'imbriquent pas). Une couche
décochée sur un travail n'est pas coupée sur la planche. Seuls les PDF s'imbriquent ; une
page plus large que la laize n'entre dans aucune planche.

### File > planche en préparation

Dans la vue **File d'attente**, la colonne d'une imprimante qui imbrique affiche
« Imbrication : *stratégie* » et, au-dessus de ses sorties, chaque **Planche en
préparation — n pages** :

- le support, la découpeuse (« aucune ») et le miroir de la planche, et **contour
  NestTrimBox** s'il est demandé ;
- le dessin de l'imposition (la laize, chaque élément à sa place) ;
- **Longueur … mm · remplissage … %** : longueur de rouleau consommée (marges
  comprises) et part de ce rouleau couverte par les éléments ; *Déclencheur atteint : la
  planche part.* quand c'est le cas ;
- la liste des pages : nom, page, copies, commande et échéance ; une case par page
  (**Imprimer cette page**) : décochée, la page sort de la planche et de la file, jusqu'à
  ce qu'elle soit recochée ;
- **Imprimer maintenant** : la planche part sans attendre son déclencheur ;
- **PDF de la planche** : le PDF exact qui sera imprimé (l'aperçu est le PDF produit).

Quand la planche part, elle devient un travail « Imbrication *machine* — n page(s) »,
rendu puis mis en file sur l'imprimante (et sur la découpeuse du groupe s'il en a une).
Sa fiche reprend les commandes posées, l'échéance la plus proche et, en notes, chaque page
avec ses copies. Les impressions posées (et la découpe en file de ces pages) passent
**Imbriquée**.

**Planche abandonnée** : rendu en échec, travail de planche supprimé, ou impression de la
planche annulée, en échec ou retirée avant d'être imprimée. Ses travaux reviennent d'eux-
mêmes en file, dans leur état d'avant (une découpe suspendue revient suspendue), et dans
la planche en préparation. Rien ne revient d'une planche imprimée. Une planche abandonnée
ne se refait plus : **Refaire** sa sortie, **Relancer** son travail ou le remettre en file
est refusé (*Planche abandonnée : ses travaux sont revenus en file…*), sinon ses travaux
s'imprimeraient deux fois ; **Imprimer maintenant** sur la planche en préparation en refait
une juste. Une planche imprimée, elle, se **Refait** (réimpression voulue).

## Couleur

Bloc **Couleur** (mode Imprimer) :
- **Ajouter un profil ICC…** dépose un profil `.icc` ou `.icm` sur le serveur (profils de
  support, profil d'entrée Epson Wide Gamut…).
- **Entrée CMJN / Entrée RVB** : profil appliqué aux données sans profil (ou à toutes,
  avec **Ignorer les profils du PDF**).
- **Support (sortie)** : profil du support imprimé ; l'aperçu simule son rendu.
- **Intention** : perceptuelle, colorimétrique relative, saturation, absolue.

Les tons directs qui ont leur encre (`White`, `Silver`…) ne sont jamais convertis. Pour
retoucher les couleurs d'un travail : [filtre couleur](#filtre-couleur) de la visionneuse.

### Table des tons directs

**Système > Tons directs** (administrateur) : un ton direct du fichier dont le nom figure
dans la table est imprimé en CMJN d'après **sa couleur Lab**, et non d'après la couleur de
secours écrite dans le PDF. La conversion passe par le profil de sortie et l'intention du
travail ; l'aperçu montre la couleur Lab. Le travail le signale (« Tons directs imprimés
en CMJN d'après la table »).

- Le nom doit être **identique** : casse et espaces comptent (« PANTONE 485 C » ne trouve
  pas « Pantone 485 C »). Un **alias** couvre une autre écriture.
- **Importer un CSV Lab** : la bibliothèque de votre fournisseur (ex. nuancier HEXIS), une
  teinte par ligne, `nom;L;a;b` (`;`, tabulation ou `,` ; virgule décimale admise avec `;`).
  Un nom déjà présent refuse tout l'import, sans rien écrire. Aucune bibliothèque n'est
  livrée avec le RIP ; n'importez que celles que vous avez le droit d'utiliser.
- `White`, `Silver`, `Metallic` et les tons de découpe ne peuvent pas entrer dans la table.
- **Intention des tons directs** : colorimétrique relative par défaut (on vise la teinte).
  Elle se règle pour le serveur dans Système > Tons directs ; un réglage ou un travail peut
  en choisir une autre (bloc Couleur, « Intention des tons directs »). Changer celle du
  serveur ne touche que les travaux restés sur « Celle du serveur », aux rendus suivants.
  Avec les profils de base de Ghostscript, perceptuelle et relative donnent le même
  résultat ; avec le profil ICC d'un support, elles diffèrent.
- Une teinte à 50 % pose la moitié de la recette CMJN de la teinte pleine.
- Un ton du fichier qui n'est ni une encre, ni un ton de découpe, ni dans la table est
  signalé sur la page (« ton direct absent de la table ») et noté dans **Tons rencontrés
  absents de la table** : **Ajouter** le fait entrer avec sa valeur Lab.
- Pris aux rendus suivants : relancer un travail déjà rendu.

## Système

Vue **Système** : version du serveur, durée de conservation des travaux (30 jours par
défaut, réglée par `EKO_RIP_CONSERVATION_JOURS`) et espace libre du disque des données,
en rouge sous 10 %. Un dossier ou un fichier d'état manquant ou abîmé y est signalé
(« Fichiers et dossiers »). Le bloc **Découpe** (administrateur) porte les préfixes de
découpe, les calques non imprimables, les tracés ouverts, le tri des tracés et le
choix **Après coupe** (voir [Découpe](#découpe)).

## Machines

**Système > Machines** : le catalogue des machines de l'atelier (une fiche par modèle :
[aide par machine](machines.md)). Pour chacune : un
identifiant (minuscules, chiffres, `-`), un nom, le modèle (Epson SC-S80600, Mutoh Kona,
Graphtec FC7000, Roland VersaCAMM VP), la laize, la connexion (série ou USB-série,
réseau TCP, périphérique comme `/dev/usb/lp0`, ou dossier), l'adresse SNMP pour lire le
média chargé, le pas de découpe (0,01 mm pour le Kona, 0,025 mm par défaut ailleurs) et
le **décalage X / Y** de la découpe (−20 à 20 mm) : l'écart mesuré sur une coupe d'essai
entre la découpe et l'impression, ajouté à chaque découpe envoyée à cette machine.
Aussi : l'**Origine X / Y impression** (Epson, pilote direct), la **Cadence (m²/h)**, le
**Pool**, et pour une découpeuse ses réglages de coupe, **Découpe retournée (180°)**,
**Une couche à la fois**, **Couper dès l'arrivée** et **Dupliquer (autre jeu de
repères)**, et pour une imprimante son **Imbrication** (voir [Imbrication dans la
file](#imbrication-dans-la-file)). **Enregistrer** applique tout de suite : les boutons d'impression et de
découpe, la file et la barre latérale suivent. Une machine qui a encore une sortie à
passer ne peut pas être retirée.

- **Epson SC-S80600** : connexion **Réseau (TCP)**, hôte = adresse de l'imprimante, port
  9100 = pilote direct ; **Dossier** = plaques copiées, rien n'est imprimé. Renseigner le
  SNMP pour arrêter la sortie sur une alerte.
- **Graphtec FC7000-75** : régler la machine en HP-GL (Menu > Interface > Command), pas de
  0,025 mm ; le repérage ARMS se règle au panneau de la machine.
- **Roland VersaCAMM VP-300 / VP-540** (762 / 1371 mm) : la découpe part par le réseau ;
  à éprouver à l'atelier. L'impression Roland viendra après une capture réseau.

**Pool d'imprimantes** : le même nom de **Pool** sur des machines identiques (même
modèle, même laize). Un bouton **Imprimer sur pool « … »** apparaît sur chaque page ; la
sortie se montre chez chaque membre et part sur la première machine prête, qui la
prend. Refusé si les supports chargés connus des membres diffèrent : charger le même
support, ou dupliquer la machine.

## Supports et modes d'impression

**Système > Supports de l'Epson** (administrateur) : le catalogue des supports. Pour
chacun : **Nom**, **N° dans l'imprimante** (le n° de support enregistré dans l'Epson,
qui permet de reconnaître le support chargé), **Laize (mm)**, **Encres**, et ses
**Modes** d'impression, écrits `Nom = 720 dpi / 8 passes ; Autre = 360 dpi / 4`. Un
troisième champ, facultatif, donne le volume des gouttes petite, moyenne et grosse pour
l'encre estimée : `Autre = 360 dpi / 4 / 5-10-15 pL`. Sans lui, 5-10-15 pL : des valeurs
de départ choisies par nous (proportionnelles aux trois tailles de point du tramage), à
étalonner. Les prix restent dans « Coûts d'encre et de support », reliés par le nom. Chaque support et
chaque mode garde un identifiant stable : le renommer ne le change pas.

Deux autres champs facultatifs, dans n'importe quel ordre après les passes, règlent la
séparation des encres claires du mode (pilote direct de l'Epson) :
`Affiche = 720 dpi / 8 / franche / 260 %`.

- **Bascule des claires** : où la foncée prend le relais de la claire (Lc, Lm, Lk).
  **douce** : claire pleine jusqu'à 30 % de densité, relais long jusqu'à 70 %, un peu de
  claire gardée sous la foncée ; le moins de grain, pour les dégradés et la peau vus de
  près. **équilibrée** (par défaut) : claire seule jusqu'à 24 %, relais jusqu'à 54 %.
  **franche** : claire limitée à 60 %, relais de 18 à 33 % ; moins d'encre et un séchage
  plus rapide, pour l'affichage vu de loin. Valeurs à nous, à confirmer sur papier.
- **Encre totale** (`260 %`, de 100 à 400 %) : la somme maximale des 9 encres en un point
  (300 % sans elle).

**Limite d'encre proposée.** Imprimer des aplats d'encre totale croissante (par exemple
de 100 à 400 % par pas de 20 %), mesurer leur L*, puis envoyer la série à
`POST /api/linearisation/limite-proposee` (`{"mesures": [[100, 58.0], [120, 47.2], …]}`).
La réponse propose la limite : le dernier palier qui fonce encore d'au moins 1 L* par
10 % d'encre en plus, arrondi aux 5 % inférieurs ; un palier qui s'éclaircit (bronzage,
bavure) l'arrête. Rien n'est enregistré : relire les gains renvoyés, vérifier l'aplat au
séchage, puis saisir la limite dans le mode.

**Mire de linéarisation** : `GET /api/linearisation/mire.cgats?n=21` (option
`encres=C,Lc,M`) donne une mire CGATS d'une plage par encre et par palier, resserrée dans
les hautes lumières : 8 plages sous 20 % sur 21, contre 4 pour une rampe régulière.

**À l'envoi**, **Support et mode d'impression** : le mode impose la résolution du rendu ;
le support relie aux tarifs. **— résolution choisie à la main —** garde la résolution du
panneau. Changer de mode sur un travail reçu refait le rendu.

**Support chargé** (Système > Média chargé dans l'Epson) : reconnu par son n° lu sur
l'imprimante, ou choisi par l'opérateur dans la liste (le choix prime jusqu'au retour à
**— lu sur l'imprimante —**). Un n° inconnu du catalogue est signalé. Une impression
attend que le support chargé soit celui du travail (**Autre support**), puis part
d'elle-même ; la laize du rouleau chargé borne la page (**Trop large**). Support chargé
inconnu : rien n'est bloqué.

**Paquet support** : **Exporter** (sur la ligne d'un support) donne un fichier avec ce
support, ses modes et les réglages qui s'en servent, sans profil ICC. **Importer un
paquet support** le reprend sur un autre serveur : identifiants gardés, réimport sans
doublon ; un paquet fait par une version plus récente est refusé (mettre ce serveur à
jour d'abord).

## Média chargé dans l'Epson

**Système > Média chargé dans l'Epson** : la largeur du rouleau chargé, que l'imprimante
mesure au chargement, est lue par le réseau (SNMP, Printer-MIB normalisée) et devient la
laize des panneaux Planche et Mosaïque (**Utiliser comme laize**, **Relire**). Il faut
renseigner l'adresse IP de l'Epson dans `EKO_RIP_EPSON_SNMP` (voir l'installation du
serveur). Validé sur la SC-S80600 : l'écran montre aussi la longueur restante du rouleau,
le n° de support, les chauffages, le niveau de chaque encre, les alertes et le
support chargé.

**Consommation réelle** : le serveur relève l'Epson chaque minute (encres et rouleau) et
garde chaque changement dans `releves/epson.jsonl` (dossier des données). L'écran affiche
ce qui a été consommé sur 7 jours ; l'API `GET /api/machines/epson/consommation?depuis=&jusqua=`
(secondes Unix) le donne pour toute période. Seules les baisses comptent : un changement de
poche ou de rouleau n'est pas compté comme une consommation. L'imprimante ne donne l'encre
qu'au % près : le chiffre est fiable sur la durée, pas pour un petit travail.

## Encre et coût

Chaque page imprimée indique sa surface, l'encre estimée et son coût :
`0,42 m² · encre 3,1 ml · ≈ 1,92 € (encre 0,44 €, support 1,48 €)`. Sous chaque
vignette de plaque, son taux d'encre (part de la page couverte à 100 %).

Le calcul : pour chaque plaque, *taux × surface × ml/m² à 100 %* donne les ml, puis
*ml ÷ 1000 × prix au litre* le coût ; le support coûte *surface × prix au m²*. Les tons
de découpe ne comptent pas.

Les tarifs se règlent dans **Système > Coûts d'encre et de support** : une ligne par
encre (ml/m² à 100 % et € par litre), une ligne **Toutes les autres** pour les cases
vides, et le prix du support au m². Les valeurs livrées (10 ml/m², cartouche Epson de 700 ml à 123,90 € TTC soit 177 €/l, support HEXIS V3000WG à 1,97 €/m² en rouleau de 1600 mm × 45 m) sont des
valeurs de départ : les remplacer par celles de l'atelier (pesée des cartouches avant
et après un tirage connu, factures). Un changement de tarif s'applique aussi aux
travaux déjà faits.

**Cartouches** : saisir la contenance (ml) et le prix d'une cartouche ; le prix au litre
en est tiré. **Supports (rouleaux)** : un catalogue, chaque support avec son prix au m²,
ou sa largeur, sa longueur et le prix payé du rouleau (le prix au m² est calculé). Le
support se choisit à l'envoi (**Support (prix de revient)**, gardé dans les réglages) ;
avec une largeur connue, la matière comptée est toute la largeur du rouleau sur la
longueur imprimée. La carte du travail affiche son **prix de revient** (encre + matière).

**HT ou TTC** : indiquer si les prix saisis sont **TTC** ou **HT**, le taux de **TVA**
(20 % par défaut) et la base du **prix de revient affiché** (HT par défaut) ; la carte,
l'estimation de chaque page et le CSV (colonne `base`) suivent.

Deux rapports : **Exporter la consommation (CSV)** donne le **réel**, une ligne par
impression du journal (voir ci-dessous) ; **Exporter les estimations (CSV)** donne
l'**estimé**, une ligne par page de chaque travail rendu, imprimé ou non : date UTC,
travail, page, support, surface, surface de support, ml estimés par encre (`BK_ml`,
`C_ml`, `M_ml`, `Y_ml`, `WH_ml`, `MS_ml`, tons directs dans `autres_ml`), coûts encre et
support, total et base. Mêmes conventions : séparateur `;`, décimales à la virgule,
colonnes fixes, ligne `total` égale à la somme, période par
`/api/rapports/estimations.csv?depuis=&jusqua=` (date de création du travail, secondes Unix).

Limite connue : sans largeur de rouleau connue, le support compté est la surface de la
page, pas la longueur de rouleau consommée sur toute la laize.

## Journal des impressions

**Système > Journal des impressions** : chaque impression terminée, la plus récente en
haut (les 200 dernières) : **Date**, **Machine**, **Travail** (et sa page), **Support**,
**Mode**, **Opérateur** (celui de la fiche travail), **Surface**, **Longueur**, **Encre
estimée**, **Encre réelle**, **Écart** et **Coût encre**. Le survol de l'encre estimée ou
réelle donne le détail par encre (BK, C, LC…).

**Encre réelle** : par le pilote direct, le serveur relève l'Epson (SNMP) juste avant
l'envoi et juste après la fin de l'impression, en plus du relevé de chaque minute. Ce qui a
baissé entre les deux est rangé dans la sortie ; la file l'affiche sur la ligne de la
sortie terminée : `Relevé : encre 14 ml, support 574 mm`. L'imprimante ne donne l'encre
qu'au % près d'une poche (1 % de 700 ml = 7 ml) :

- une impression qui ne voit aucune baisse affiche **< 7 ml**, jamais 0 ;
- une baisse de 1 % s'est faite depuis la variation précédente du même canal : elle est
  partagée entre toutes les impressions faites depuis, au prorata de leur encre estimée
  (à défaut, de leur durée). Un chiffre isolé reste approximatif ; la somme sur une
  période est fiable. Une baisse hors de toute impression (nettoyage) n'est attribuée à
  personne ; un changement de poche n'est pas une consommation.

La contenance d'une poche est celle de la cartouche saisie dans « Coûts d'encre et de
support » (ligne de l'encre, de sa plaque ou **Toutes les autres**), 700 ml sinon. Sans
adresse SNMP, l'encre réelle est **non mesuré**.

**Longueur** : `369 → 574 mm (gâche 204 mm)` : la longueur de la page, celle relevée sur
le rouleau entre le début et la fin, et la différence (amorce, tête vide, marges).
🏭 À vérifier à l'atelier sur un vrai tirage (le 24/10 : travail de 369,8 mm, 574 mm
relevés, soit 204 mm de gâche).

**Encre estimée** : par le pilote direct, les points posés par le tramage sont comptés
encre par encre et taille par taille ; *ml = Σ points × volume de goutte du mode* (voir
« Supports et modes d'impression »), multiplié par le facteur de correction validé. Une
sortie en dossier (plaques en ton continu, tramées par un autre logiciel) est **non
calculable** : ni encre ni coût encre, plutôt qu'un chiffre faux. L'estimation affichée
sur la carte du travail avant impression reste celle de la couverture de l'aperçu.

**Rapports CSV** : **Rapport CSV par impression** (une ligne par impression) et **Rapport
CSV par jour** (une ligne par jour et par machine), avec une dernière ligne `total` égale
à la somme des lignes. Séparateur `;`, décimales à la virgule, colonnes toujours les
mêmes et dans le même ordre : identité (date UTC, machine, travail, page, nombre
d'impressions, support, mode, dpi, opérateur), surface, longueurs prévue et réelle,
gâche, méthode d'estimation (`points` ou `non calculable`), encre estimée et réelle
côte à côte (totaux, `sous_seuil`, écart en %, puis une paire `<encre>_estime_ml` /
`<encre>_reel_ml` pour chacune des 11 encres du catalogue), coûts et base HT/TTC.
L'adresse `/api/rapports/consommation.csv?depuis=&jusqua=` (secondes Unix) limite à
une période.

**Rétention (jours)** (administrateur) : les impressions plus anciennes sortent du
journal, 90 jours par défaut ; la purge passe toutes les 5 minutes. **Purger le
journal** demande combien de derniers jours garder (0 : tout effacer). Les travaux et la
file ne sont jamais touchés. Les relevés de l'imprimante restent.

**Facteurs de correction de l'encre estimée** (administrateur) : un facteur par machine,
résolution et encre, 1 par défaut. Le serveur propose *encre réelle ÷ encre estimée* sur
les 10 dernières impressions mesurées de chaque combinaison ; **Appliquer** le valide,
**Remettre à 1** l'enlève. Il ne vaut que pour les impressions suivantes : le journal garde
le facteur de chacune. Rétention et facteurs sont dans la sauvegarde de la configuration.

## Ce qui n'existe pas encore

- Le blanc et l'argent par le pilote direct de l'Epson : ils sortent en dossier.
- L'impression par la Roland.
- Le cadre Kona type 3 (multi-segments) n'a pas encore été essayé sur la machine.
- Les canaux de tons directs d'un TIFF : le sens des valeurs reste à vérifier sur un vrai
  TIFF Photoshop ; la transparence comme plaque de blanc viendra avec la roadmap 9.12.
- Les couleurs métallisées `MT-*` du nuancier Epson ne sont pas encore converties.

## Glossaire

Les mots de l'écran, avec leur équivalent anglais : [glossaire](glossaire.md).
