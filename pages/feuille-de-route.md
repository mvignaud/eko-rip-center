---
titre: Feuille de route
description: Ce que le RIP libre E-KO RIP Center a fait, ce qui est en cours et ce qui est planifié. Page publique, mise à jour à chaque version, sans date promise.
maj: 2026-11-03
---
La feuille de route d'E-KO RIP Center est publique. Nous la tenons à jour à chaque version : une fonction passe de « Planifié » à « En cours », puis à « Fait ». Les dates ne sont pas promises, car beaucoup de points attendent un essai sur une vraie machine à l'atelier. Il vous manque une fonction ? Demandez-la par [/contact/](/contact/). Il vous manque une machine ? Dites-le sur [/compatibilite/](/compatibilite/). Le code et les tickets sont sur [GitHub](https://github.com/mvignaud/eko-rip-center).

## Fait

### File et production
- **Écran de production unique** : vos machines, les travaux à imprimer et les travaux imprimés tiennent sur une seule vue, avec jauges d'encre, clic droit et file réordonnable au glisser-déposer.
- **Dossiers surveillés à règles et noms balisés** : un fichier déposé prend tout seul le bon réglage, le bon nombre de copies ou la bonne machine, d'après son nom ou sa taille.
- **TIFF, JPEG et PNG en entrée** : vous imprimez directement vos images, avec leurs canaux de tons directs, et la transparence peut devenir une plaque de blanc ou de vernis.
- **Très grands formats** : une bâche de 1,6 × 3 m à haute résolution se rend par bandes, sans saturer la mémoire du serveur.

### Couleur
- **Séparation sur 9 encres et tramage** : les encres claires, l'orange et le rouge de l'Epson sont utilisés, avec des courbes de passage clair-foncé au choix (douce, équilibrée, franche).
- **Table de tons directs** : un ton nommé dans le fichier est converti en recette de l'atelier, et les noms inconnus sont signalés plutôt que mal imprimés.

### Imposition et imbrication
- **Imbrication dans la file** : les travaux de même support, même mode et même découpeuse se regroupent sur une planche, déclenchée à la main, à l'heure, à la surface ou après inactivité.
- **Éditeur de travail** : zoom, règles, recadrage, fond perdu, œillets, repères, étiquette en marge, annuler et rétablir, pour préparer un fichier sans repasser par un autre logiciel.

### Découpe
- **Couches de découpe par nom** : chaque ton de découpe reçoit le réglage de coupe du même nom, dans l'ordre que vous choisissez, et l'impression est refusée avant tout gâchis d'encre si la découpeuse n'est pas disponible.
- **Print & cut Mutoh Kona** : cadre de repérage et code-barres générés, découpe lancée une fois l'impression terminée.

### Pilotes de machines
- **Pilote direct Epson SureColor SC-S80600** : envoi par le réseau, état du support, des encres et des alertes lu sur la machine.

### Clients et applications
- **Clients Mac et Linux natifs, application mobile** : l'atelier se suit depuis le poste ou le téléphone, avec notifications quand une machine s'arrête, manque d'encre ou de support.

### Cloud et accès à distance
- **Accès à distance sans VPN ni port ouvert** : vous retrouvez votre atelier depuis l'extérieur avec un compte, sans toucher à la box de l'atelier.

### Rapports et coûts
- **Consommation réelle par travail** : encre en millilitres, longueur de support et gâche sont relevées sur la machine et comparées à l'estimation, avec export CSV.

### Lien avec l'ERP
- **Pont complet avec votre logiciel de gestion** : envoi de travaux par référence, suivi par événements rejouables après une coupure, estimation avant impression et consommation réelle remontée.

### Administration
- **Fichiers d'état protégés et sauvegarde avant mise à jour** : une coupure de courant ne vide plus la file, et la configuration est mise de côté avant chaque mise à jour du paquet.

## En cours

### File et production
- **Impression pendant le rendu** : l'envoi à la machine démarre dès les premières bandes prêtes, ce qui raccourcit l'attente sur les grands formats.
- **Formats d'entrée EPS, PSD et PostScript** : vous déposez ces fichiers sans les convertir en PDF au préalable.
- **Envoi d'un travail vers un autre serveur RIP** : vous copiez ou déplacez un travail vers la file d'un serveur de l'atelier ou d'un autre site.
- **Copies demandées et copies faites** : la file indique combien d'exemplaires restent à sortir, utile pour les reliquats.
- **Recherche par période dans l'archive** : vous retrouvez, réimprimez ou supprimez un travail ancien en filtrant par date.
- **Rendu réparti sur plusieurs serveurs** : deux postes se partagent le calcul quand beaucoup de fichiers arrivent en même temps.

### Couleur

- **Conversion de couleur entre profils sans perte (DeviceLink)** : une transformation directe, fixée une fois, pour les travaux répétitifs.
- **Restriction d'encre mesurée** : une mire lue au spectrophotomètre fixe la quantité maximale de chaque encre pour chaque support.
- **Linéarisation par canal** : les rampes d'encre sont corrigées d'après les mesures, avec historique des passes conservé.
- **Limites d'encre combinées** : l'encre totale, les doubles et les neutres sont plafonnés pour éviter bavures et temps de séchage excessif.
- **Profils ICC créés à l'atelier** : vous fabriquez vos profils vous-même avec votre spectrophotomètre, sans dépendre d'un fichier fourni par un tiers.
- **Nuanciers mesurés** : vous imprimez, mesurez et enregistrez vos nuanciers par support, et l'écran indique ce qui attend encore une mesure.
- **Contrôle de constance** : le RIP compare une impression de référence à l'impression du jour et vous dit si la machine a dérivé.
- **Vérification de conformité aux normes d'impression** : vous contrôlez vos épreuves face aux références Fogra, G7 et ISO 12647 avec un écart de couleur chiffré.
- **Étalonnage du gris appliqué au rendu** : les courbes de correction calculées sont utilisées à chaque impression pour des gris neutres.
- **Économie d'encre** : le gris est remplacé par du noir sur un profil existant pour réduire la consommation sans changer l'aspect.
- **Bibliothèques de tons directs fournies** : les nuanciers de tons directs courants sont livrés avec le RIP, sans remplir la table à la main.
- **Contrôle couleur dans le cloud** : vous suivez la conformité couleur de vos impressions depuis le Cloud, pour un ou plusieurs sites.
- **Recette 9 encres des tons directs** : le RIP cherche la meilleure combinaison d'encres pour approcher un ton direct et affiche l'écart estimé.
- **Lecture des fichiers de supports Epson** : résolution, passes et chauffage se pré-remplissent depuis les réglages enregistrés dans l'imprimante.

### Imposition et imbrication
- **Planche manuelle à l'écran** : vous glissez, verrouillez et calez vos copies sur une grille depuis les clients natifs.
- **Gabarits de cellules et données variables à l'écran** : une copie par ligne d'un fichier CSV, avec textes personnalisés, directement dans le client.
- **Lés inégaux et tuilage en colonnes et rangées** : vous déplacez les frontières des lés à la souris pour découper une grande image en panneaux de largeurs différentes.
- **Recouvrement et espace propres à chaque tuile** : un panneau peut avoir un recouvrement plus large que les autres.
- **Ordre d'impression des tuiles** : vous décidez dans quel ordre les panneaux sortent, pour la pose ou pour économiser le support.
- **Barre de couleurs et bande de contrôle en marge** : vous imprimez vos repères de contrôle avec le travail pour vérifier la couleur à la sortie.
- **Code-barres 1D en marge** : en plus du QR code, un code-barres lisible par vos lecteurs d'atelier.
- **Repères de massicot** : les traits de coupe pour massicot automatique sont générés avec leur espacement et leur épaisseur.

### Découpe
- **Matière de découpe associée au support** : choisir le support suffit à régler la matière sur les découpeuses qui en proposent.
- **Fichiers pour tables de découpe à plat** : vos contours partent dans les formats attendus par les tables de découpe industrielles.
- **Repères blancs sur support transparent ou foncé** : un fond blanc est posé sous les repères pour qu'ils restent lisibles par la découpeuse.
- **Repérage Kona pour grands rouleaux (cadre multiple)** : le cadre avec repères intermédiaires est à valider sur un rouleau de 1,5 m.
- **Miroir des tracés en print & cut** : le comportement du miroir sur impression et découpe est à vérifier à l'atelier avant de le proposer.
- **Découpe Graphtec et Roland éprouvée** : un carré de 100 mm est à couper et mesurer sur chaque machine.

### Pilotes de machines
- **Blanc et argent sur l'Epson** : les encres spéciales partent directement vers la machine, après une capture d'un travail réel.
- **Autres résolutions d'impression** : 360 × 720, 720 × 1440 et 1440 × 1440, avec protection de la rotation automatique quand les points ne sont pas carrés.
- **Impression sur Roland VersaCAMM** : l'impression complète la découpe déjà en essai, une fois le flux de la machine étudié.
- **Imprimantes d'autres marques** : nous ajoutons les imprimantes grand format une à une, selon vos demandes sur la page compatibilité.
- **Détection des machines sur le réseau** : le RIP repère les imprimantes de l'atelier et propose de les ajouter.

### Clients et applications
- **Client Windows natif** : le même poste de conduite que sur Mac et Linux ; un premier installateur est publié, il reste à le compléter.
- **Installateur Windows signé** : le client Windows s'installe sans l'avertissement SmartScreen, comme le client Mac, déjà signé et notarisé.
- **Écrans d'administration natifs** : supports, règles, tarifs, tons directs, machines et sauvegarde s'ouvrent dans l'application, sans fenêtre web.
- **Éditeurs visuels natifs** : planche d'imbrication, détourage et mise en coupe s'utilisent dans l'application.
- **Plusieurs serveurs dans un même client** : une seule fenêtre, les files de tous vos sites, un envoi vers le serveur de votre choix.
- **Réglages récents dans les clients** : lissage des images agrandies, empilement de couches et rappel de recalibration par machine.

### Rapports et coûts
- **Consommation réelle éprouvée à l'atelier** : la gâche et les millilitres relevés sont comparés à des impressions mesurées à la main.

### Lien avec l'ERP
- **Pont éprouvé en production** : les échanges avec le logiciel de gestion sont validés sur de vraies commandes, pas seulement sur un banc d'essai.
- **Échange JDF et JMF** : les logiciels de gestion qui parlent ce format peuvent envoyer des travaux et recevoir des comptes rendus.

## Planifié

### File et production
- **Rouleau actif sur machine multi-rouleaux** : le RIP sait quel rouleau est chargé et envoie le travail sur le bon.
- **Dossier surveillé en copie** : le fichier d'origine reste en place au lieu d'être déplacé après traitement, au choix du réglage.
- **Option « ne pas ripper les travaux en attente de support »** : le serveur économise son calcul tant que le bon support n'est pas chargé.
- **Imprimante virtuelle sur les postes** : vous imprimez depuis n'importe quel logiciel vers le RIP comme vers une imprimante ordinaire.
- **Envoi de fichiers depuis le téléphone** : vous déposez un fichier dans la file depuis l'application mobile.

### Couleur
- **Import de nuanciers au format CxF** : vous chargez les nuanciers de vos fournisseurs sans passer par un tableur.
- **Profil d'entrée CMJN par défaut documenté** : un choix de départ clair pour les fichiers sans profil intégré.
- **Cibles de linéarisation plus fines dans les hautes lumières** : les dégradés clairs sont mieux tenus.
- **Recettes de couleurs métallisées mesurées** : combinaisons d'argent et d'encres pour approcher une cible métallisée.
- **Profils spectraux hybrides** : une meilleure prévision des couleurs sur supports très variables.
- **Compensation de dérive entre lots d'encre** : le RIP propose un recalage quand vous changez de lot.

### Imposition et imbrication
- **Zone de soudure sans encre** : la bande de recouvrement des bâches soudées reste sans encre, avec choix des lés à imprimer.
- **Coupe au milieu du recouvrement** : le trait de coupe passe au centre de la zone commune de deux lés voisins.
- **Rapport PDF de mise en page** : un document récapitulatif de la planche à joindre au dossier de fabrication.
- **Imposition recto-verso** : placement de deux faces d'un même travail pour les supports imprimés des deux côtés.

### Découpe
- **Découpe de rouleau à rouleau** : vous coupez un rouleau imprimé sans le recouper en feuilles.
- **Correspondance automatique de la matière** : le RIP propose la matière de la découpeuse d'après le nom du support.
- **Tons de finition** : vernis sélectif, dorure à froid, perforation et numérotation reconnus comme des opérations à part.
- **Lecture de fichiers de découpe en entrée (DXF)** : vous réutilisez un contour dessiné dans un logiciel de CAO.

### Pilotes de machines
- **Cadences réelles par mode** : la durée annoncée repose sur des mesures de votre atelier plutôt que sur les chiffres du fabricant.
- **Lecture du bac d'encre usagée** : le niveau du bac est affiché et transmis dans l'état de la machine.

### Clients et applications
- **Fenêtre d'état du serveur dans les clients** : espace disque, rendus en cours et alertes visibles d'un coup d'œil.

### Cloud et accès à distance
- **Alertes du cloud dans les clients** : les messages du site hors ligne ou revenu apparaissent aussi dans la fenêtre de l'application.
- **Choix du site par défaut** : vous ouvrez directement l'atelier que vous suivez le plus souvent.
- **Dépôt de fichiers par le cloud** : vous envoyez un travail à l'atelier depuis l'extérieur, sans passer par le réseau local.

### Rapports et coûts
- **Étiquettes 100 × 150 mm** : imprimer à la demande une étiquette de travail fini, d'expédition ou de rouleau en stock, avec code-barres.
- **Facteur de correction par résolution** : l'estimation d'encre se recale seule sur vos mesures réelles après quelques travaux.
- **Tableau de bord de production** : suivi en direct et historique de l'activité par machine, par jour et par opérateur.

### Lien avec l'ERP
- **État machine à la demande** : votre logiciel de gestion interroge le support chargé, les niveaux d'encre et les alertes quand il en a besoin.
- **Devis automatique d'encre et de support** : une estimation de quantités fournie pour chaque demande de prix avant même la commande.

### Administration
- **Rapport de migration** : à chaque mise à jour, un résumé indique ce qui a été repris ou changé.
- **Dossiers de travail relocalisés** : si un dossier disparaît, le RIP propose de le recréer ailleurs et vous prévient.
- **Journal d'activité par opérateur** : qui a lancé, suspendu ou supprimé quoi, consultable par l'administrateur.
