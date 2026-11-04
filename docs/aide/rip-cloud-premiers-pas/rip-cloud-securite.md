---
titre: Comprendre la sécurité du cloud
description: Ce qui protège votre atelier : aucun port ouvert, adresses isolées par site, blocage des mots de passe et sessions séparées.
sous_rubrique: rip-cloud-premiers-pas
position: 23
mots_cles: sécurité, cookies, clé du site, mot de passe, isolation, confidentialité
---

Le cloud est conçu pour ne jamais ouvrir l'atelier à des inconnus.
Cet article résume les protections, sans configuration de votre part.

## Avant de commencer

- Rien à régler : ces protections sont actives dès que le serveur est relié.
- Pour le détail des conditions d'utilisation, voir [Consulter les conditions et les mentions légales](rip-cloud-consulter-les-informations-legales.md).

## Étapes

1. **Aucun port ouvert.** Le serveur se connecte à la plate-forme, jamais l'inverse. Vous ne changez rien à votre box.
2. **Une clé par site.** Au rattachement, le serveur reçoit une clé secrète qu'il garde dans un fichier protégé ; elle n'est jamais réaffichée. La plate-forme ne conserve qu'une empreinte de cette clé. Une clé fausse et un site inconnu reçoivent la même réponse.
3. **Une adresse isolée par site.** Chaque site a sa propre adresse (`https://<site>.platform.e-ko.rip`). Le navigateur sépare ses cookies et ses pages de ceux des autres sites et du portail. Le portail n'existe pas sur l'adresse d'un site.
4. **Seule la session du serveur RIP part vers l'atelier.** Le portail ne transmet à l'atelier ni les informations de votre compte de la plate-forme, ni ses cookies.
5. **Mots de passe protégés.** Après 5 échecs de connexion, le mot de passe d'un compte est bloqué 5 minutes. Les formulaires n'acceptent que les demandes venues de la plate-forme elle-même.
6. **Accès des apps limité.** L'accès d'une app dure environ 180 jours et se révoque à tout moment. Une nouvelle app connectée déclenche toujours un e-mail de sécurité.

## Bon à savoir

- Les sauvegardes en ligne sont chiffrées et hébergées en France.
- Si vous perdez un appareil, utilisez **Déconnecter toutes les apps** dans **Mon compte**. Voir [Déconnecter les apps du cloud](../rip-cloud-compte/rip-cloud-deconnecter-les-apps.md).
- Un nouveau mot de passe ferme toutes les sessions et déconnecte les apps.
- Un site hors ligne ou inconnu reçoit la même réponse : le cloud ne révèle pas quels sites existent.
- Un envoi de fichier de plus de 2 Go par le cloud est refusé.

## Voir aussi

- [Comprendre le cloud E-KO RIP](rip-cloud-comprendre-le-cloud.md)
- [Savoir ce qui ne passe jamais par le cloud](rip-cloud-ce-qui-reste-reserve-a-l-atelier.md)
- [Déconnecter les apps du cloud](../rip-cloud-compte/rip-cloud-deconnecter-les-apps.md)
