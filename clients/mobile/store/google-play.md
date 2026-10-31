# Fiche Google Play — E-KO RIP

Package : `fr.eko.rip.mobile`. Langue par défaut : français (France) ; traduction : anglais (États-Unis).
Visuels : `store/captures/android/` (captures 01 à 04, `feature-graphic.png` 1024x500). Icône 512x512 : à exporter depuis `assets/icon.png`.

## Français (fr-FR)

**Titre** (≤ 30) : `E-KO RIP`

**Description courte** (≤ 80) :
`Suivez la file d'impression, les encres et les alertes de vos machines RIP.`

**Description complète** (≤ 4000) :

```
E-KO RIP est l'application compagnon du RIP E-KO, le serveur d'impression grand format de votre atelier. Elle vous permet de suivre la production à distance, depuis votre téléphone.

FILE D'ATTENTE
• Les sorties de chaque machine : en cours, en attente, suspendues, terminées ou en échec
• Durée écoulée et durée estimée, message d'erreur en cas d'échec
• Actions rapides : lancer ou reprendre, mettre en pause, passer en tête, arrêter, relancer une sortie

MACHINES
• État de chaque machine : en impression, en découpe, en attente, injoignable
• Niveau de chaque encre, avec mise en évidence des niveaux bas
• Longueur de support restante sur le rouleau
• Alertes de la machine (capot ouvert, fin de rouleau…)

ALERTES ET NOTIFICATIONS
• Notifications push en cas d'erreur machine, d'encre basse ou de sortie en échec
• Fins d'impression en option
• Historique des dernières alertes, avec pastille de nouveautés

À SAVOIR
• L'application se connecte au serveur RIP E-KO de votre entreprise : il faut être sur le réseau de l'atelier ou connecté au VPN.
• Aucun compte E-KO n'est nécessaire pour découvrir l'application : touchez « Essayer le mode démo » sur l'écran de connexion.
• Vos identifiants restent sur votre téléphone et ne sont envoyés qu'à votre propre serveur.
• L'application ne permet ni l'envoi de fichiers ni le réglage de l'impression ou de la découpe : elle sert à surveiller et à piloter la file.
```

## English (en-US)

**Title** (≤ 30): `E-KO RIP`

**Short description** (≤ 80):
`Follow the print queue, ink levels and alerts of your RIP-driven machines.`

**Full description** (≤ 4000):

```
E-KO RIP is the companion app for the E-KO RIP, the large-format print server of your shop. It lets you follow production remotely from your phone.

PRINT QUEUE
• Outputs for each machine: running, waiting, paused, done or failed
• Elapsed and estimated time, error message when a job fails
• Quick actions: start or resume, pause, move to front, stop, reprint an output

MACHINES
• Status of each machine: printing, cutting, idle, unreachable
• Level of every ink, with low levels highlighted
• Media length left on the roll
• Machine alerts (cover open, media near end…)

ALERTS AND NOTIFICATIONS
• Push notifications for machine errors, low ink and failed outputs
• Optional notifications when a print finishes
• History of recent alerts with an unread badge

GOOD TO KNOW
• The app connects to your company's own E-KO RIP server: you must be on the shop network or connected to its VPN.
• No E-KO account is needed to try the app: tap "Essayer le mode démo" (demo mode) on the login screen.
• Your credentials stay on your phone and are only sent to your own server.
• The app does not upload files or change print or cut settings: it is for monitoring and managing the queue.
```

## Classification

- Type : application. Gratuite. Catégorie : **Outils** (alternative : Productivité). Étiquettes : professionnel / outils.
- Coordonnées : e-mail mathieu.vignaud2@gmail.com, site https://e-ko.fr.
- URL de la politique de confidentialité : https://e-ko.fr/confidentialite-rip (à confirmer ; texte dans `store/confidentialite.md`).

## Accès à l'application (Contenu de l'application > Accès à l'application)

Choisir « Toutes ou certaines fonctionnalités sont limitées » et fournir les instructions : « L'application se connecte à un serveur sur le réseau local d'une entreprise, inaccessible aux relecteurs. Sur l'écran de connexion, toucher "Essayer le mode démo" (en bas) : aucun compte requis, données fictives. »

## Questionnaire de classification du contenu (IARC)

Catégorie : « Utilitaire, productivité, communication ou autre ». Réponses : violence NON ; contenu sexuel NON ; langage grossier NON ; substances contrôlées NON ; jeux d'argent NON ; contenu généré par les utilisateurs NON ; partage de position NON ; achats numériques NON ; accès libre au web NON. Résultat attendu : PEGI 3 / Tous publics (ESRB Everyone).

## Public cible

Tranche d'âge : **18 ans et plus** uniquement (usage professionnel). Pas destiné aux enfants ; non inscrit au programme « Famille ».

## Annonces / Publicité

L'application ne contient pas de publicités : NON.

## Sécurité des données (Data safety)

- **L'application collecte ou partage-t-elle des données utilisateur ?** Oui, au sens de Google (identifiants d'appareil / jeton de notification transmis à des tiers pour le fonctionnement), aucune donnée n'est envoyée au développeur.
- **Données collectées / partagées** :
  | Type | Collecté | Partagé | Finalité | Facultatif |
  |---|---|---|---|---|
  | Identifiants d'appareil ou autres (jeton de notification push, nom de l'appareil) | Oui | Oui : serveur RIP de l'utilisateur et service de notifications Expo | Fonctionnalités de l'application (notifications) | Oui (notifications désactivables) |
  | Informations de compte (nom d'utilisateur, mot de passe du serveur RIP) | Non au sens de Google : stockés uniquement sur l'appareil, envoyés au seul serveur choisi par l'utilisateur | — | Authentification | — |
  | Tout autre type (position, contacts, photos, fichiers, finances, santé, messages, activité, historique web, plantages/diagnostics) | Non | Non | — | — |
- Adresse du serveur, identifiants : stockés sur l'appareil (stockage sécurisé), jamais envoyés au développeur.
- **Données chiffrées en transit ?** Non systématiquement : l'utilisateur peut se connecter en HTTP à un serveur de réseau local (le trafic reste sur son réseau) ; HTTPS pris en charge. Répondre « Non » à « toutes les données sont chiffrées en transit » ou préciser selon la configuration du serveur.
- **Possibilité de demander la suppression des données ?** Oui : se déconnecter ou couper les notifications retire le jeton du serveur et efface les données locales ; désinstaller l'application supprime le reste. Contact : mathieu.vignaud2@gmail.com.
- Aucune vente de données, aucune publicité, aucun suivi, aucun SDK d'analyse.
- Permissions Android : notifications (POST_NOTIFICATIONS), Internet, vibration. Aucune permission sensible.

## Notes de version (fr-FR / en-US)

fr : `Première version de E-KO RIP : file d'attente, machines (encres, rouleau, alertes), notifications push des erreurs machine et mode démo.`
en : `First release of E-KO RIP: print queue, machines (inks, roll, alerts), push notifications for machine errors and a demo mode.`
