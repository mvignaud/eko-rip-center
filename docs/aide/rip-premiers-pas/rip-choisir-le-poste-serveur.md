---
titre: Choisir le poste serveur
description: Choisissez le PC, le disque et le réseau qui feront tourner le serveur RIP sans interruption.
sous_rubrique: rip-premiers-pas
position: 1
mots_cles: serveur, pc, configuration, disque, ssd, onduleur, prérequis
---

Le serveur RIP reçoit les fichiers, calcule les rendus et pilote les machines. Cet article vous aide à choisir le PC qui le fera tourner, avant l'installation.

## Avant de commencer

Prévoyez un PC 64 bits (x86) qui restera allumé en permanence. Comptez quelques minutes pour choisir, plus le temps de réserver des adresses sur votre routeur.

## Étapes

1. Choisissez Ubuntu 26.04 LTS pour un atelier en production. Ubuntu 24.04 LTS ou Debian 13 conviennent pour des essais.
2. Prévoyez 4 cœurs ou plus et 16 à 32 Go de mémoire pour la production. Pour des essais, 2 cœurs et 16 Go suffisent.
3. Réservez un disque pour les données : un SSD dédié de 1 To ou plus en production, 500 Go pour des essais. Montez-le avant la première mise en route.
4. Branchez le PC en câble réseau, jamais en Wi-Fi.
5. Branchez le serveur, le switch et si possible la box sur un onduleur.
6. Sur votre routeur, réservez une adresse fixe pour le serveur, pour chaque imprimante en réseau et pour chaque découpeuse en réseau.

## Bon à savoir

- Un rendu pèse environ 1 Go par m² à 600 dpi (le quart à 300 dpi, quatre fois plus à 1 200 dpi). Un disque dédié évite qu'un gros rendu remplisse le système.
- Sous 5 Go libres, le serveur efface les plus vieux travaux inactifs et refuse un travail qui ne tiendrait pas. Sous 20 Go, chaque nouveau travail porte l'alerte « disque presque plein ».
- Une coupure de courant arrête le rendu en cours (le travail est à relancer) et une sortie en cours d'envoi (à refaire). Les fichiers de réglage résistent à la coupure.
- Plus de mémoire et de cœurs raccourcissent les rendus de bâches de plusieurs dizaines de m² à haute résolution.

- Le serveur de test de l'atelier E-KO tourne sur un Pentium G645T (2 cœurs, 2012), 16 Go de mémoire et un disque mécanique de 640 Go sans partition à part : il suffit aux essais, c'est un plancher, pas une cible. Les chiffres ci-dessus sont un conseil, pas une mesure : le délai d'un rendu (`EKO_RIP_DELAI_S`, 30 minutes par défaut) dit vite si le poste est trop juste.
- Comptez au moins 500 Go, plus si vous gardez les travaux longtemps (`EKO_RIP_CONSERVATION_JOURS`). La vue **Production** montre le tampon de la file et la part du disque occupée. Un disque dédié se change ou s'agrandit sans réinstaller ; montez-le sur `/var/lib/eko-rip` (ou pointez `EKO_RIP_DATA` dessus).
- Choisissez un onduleur qui tient quelques minutes et prévient le serveur pour qu'il s'éteigne proprement (par exemple avec `nut`).
- Un port USB libre est nécessaire par découpeuse en USB (Kona) ; le service est dans le groupe `dialout`.
- Pour comparer, voir aussi la page [Pré-requis](https://e-ko.rip/prerequis/) du site.

## Voir aussi

- [Installer le serveur RIP](rip-installer-le-serveur.md)
- [Se connecter au serveur depuis un poste](rip-se-connecter-au-serveur.md)
- [Brancher l'Epson SureColor SC-S80600](../rip-machines/rip-brancher-epson-sc-s80600.md)
- [Connaître les ports réseau du serveur](rip-connaitre-les-ports-reseau-du-serveur.md)
