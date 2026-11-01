# Rapports de bug

## Signaler un problème

- **Mac** : menu *Aide › Signaler un problème…* ; **Linux** : menu principal ›
  *Signaler un problème…* ; **administration** (navigateur ou fenêtre Administration) :
  *Système › Rapports de bug › Signaler un problème…*.
- Titre, description, e-mail facultatif, et case *Joindre le journal du serveur* (cochée).
- Le poste envoie à son serveur RIP (`POST /api/rapports`), qui joint son journal masqué et
  transmet à la plate-forme E-KO RIP. Réponse : une référence `R-XXXXXXXX`.

**Le titre et la description sont publiés sur GitHub** (issue du dépôt, destiné à devenir
public) : rien de confidentiel. Le journal et l'e-mail restent privés sur la plate-forme.

## Rapports automatiques (sur accord)

À la première ouverture de l'administration, le serveur demande s'il peut envoyer
automatiquement ses rapports d'erreur ; on change d'avis dans *Système › Rapports de bug*.
Tant que personne n'a répondu oui, rien ne part.

Ce qui déclenche un rapport (un même code au plus une fois par 6 h) :

| Nature | Code | Quand |
|---|---|---|
| `plantage` | `panique` | le serveur s'arrête sur une erreur interne (écrit avant l'arrêt, envoyé au redémarrage) |
| `erreur` | `rendu_echoue`, `decoupe_illisible` | rendu d'un travail en échec |
| `erreur` | `sortie_echouee` | sortie vers une machine en échec (pas les alertes de l'imprimante ni une machine éteinte) |
| `lenteur` | `rendu_lent` | rendu de plus de 30 min (`EKO_RIP_LENTEUR_MIN`) |

Contenu : version, système, code, détail de l'erreur et les 300 dernières lignes de
`journalctl -u eko-rip-serveur` (le service est dans le groupe `systemd-journal` pour les lire),
**masqués** : chemins, noms de fichiers, textes entre guillemets (noms de travaux ou de clients)
et e-mails remplacés. En attente dans `rapports/attente/` tant que la plate-forme est
injoignable.

## Côté plate-forme

`POST /api/rapports` : rapports gardés 90 jours dans `rapports/<référence>.json`, visibles dans
*Administration › Rapports* (journal compris). Au plus 20 rapports par installation et 500 en
tout par jour.

Issue GitHub : ouverte en arrière-plan si *Administration › Issues GitHub* a un dépôt
(`propriétaire/nom`) et un jeton à grain fin limité à ce dépôt (droit *Issues : lecture et
écriture*). Elle ne porte que le titre, la description (citée, mentions `@` et `#` neutralisées),
le poste, l'OS et la référence. Les rapports automatiques de même signature (poste, nature,
code) partagent une issue, complétée d'un commentaire de compteur au plus une fois par jour.
