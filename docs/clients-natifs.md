# Clients natifs (Mac, Linux, plus tard Windows)

Décision du 30/10/2026 : le client n'est plus une page HTML dans une fenêtre Tauri mais un
vrai logiciel par système, qui parle à l'API JSON du serveur.

| Système | Dossier | Pile | État |
|---|---|---|---|
| macOS | `clients/mac/` | SwiftUI (Swift Package, macOS 14+) | lot 1 |
| Linux (Ubuntu) | `clients/linux/` | Rust, GTK 4 + libadwaita | lot 1 |
| Windows | — | WinUI 3 | en attente d'un PC Windows |

Le client Tauri (`client/`) reste livré tant que les nouveaux ne font pas tout.
Icône commune : `clients/icone/` (« Rosace », PNG de 16 à 1024 px et SVG source).

## Direction visuelle

Celle de la maquette validée par Mathieu le 30/10/2026 : fenêtre à trois volets, barre
latérale (Production : Travaux ; Atelier : Machines ; Serveur : Système), barre d'outils
unifiée (Envoyer, Imprimer, Suspendre, Épreuve, recherche), file des travaux en tableau au
centre, inspecteur à droite (aperçu sur fond gris neutre et opaque, onglets segmentés).
Contrôles, police et couleurs du système, mode clair et sombre automatiques. Le verre
(matériaux translucides) seulement sur la barre latérale et la barre d'outils, jamais sous
un aperçu. Interface en français (l'anglais viendra plus tard).

## Lot 1 : ce que fait chaque client

1. **Connexion** : adresse du serveur (`hôte:8640`, retenue), puis `GET /api/session`.
   Si `acces_libre` est faux et `utilisateur` nul : écran de connexion,
   `POST /api/session` `{"nom","mot_de_passe"}` → cookie `eko_rip_session` (12 h), à
   garder et renvoyer. 401 sur n'importe quelle route → retour à l'écran de connexion.
   Menu « Changer de serveur… » (⇧⌘S) et « Se déconnecter » (`DELETE /api/session`).
2. **Travaux** : tableau de `GET /api/travaux` (plus récents en haut), rafraîchi toutes
   les 3 s. Colonnes : nom, dossier ERP (`erp.ref_erp` s'il existe, sinon `fiche.commande`),
   pages, statut (`en_attente`, `en_cours`, `termine`, `echec` + `code_erreur`), date.
   Recherche sur le nom et le dossier.
3. **Inspecteur** du travail choisi :
   - aperçu `GET /api/travaux/{id}/pages/{n}/apercu.png` (page 1, flèches si plusieurs) ;
   - fiche (`fiche` : client, commande, quantité, échéance, notes), format de page
     (`pages[].largeur_mm`/`hauteur_mm`), plaques (`pages[].plaques`), alertes
     (`alertes[].code` + `detail`) ;
   - sorties de ce travail dans la file (`GET /api/file`, `travail` = id) avec leur état ;
   - actions : **Imprimer** (`POST /api/file` `{"travail","page","machine"}` sur une
     machine choisie parmi celles qui impriment), **Relancer** (`POST /api/travaux/{id}/relancer`),
     **Épreuve TIFF** (télécharge `GET /api/travaux/{id}/pages/{n}/epreuve.tif` et propose
     de l'enregistrer), **Supprimer** (`DELETE /api/travaux/{id}`, avec confirmation).
4. **Envoi** : bouton « + », glisser-déposer sur la fenêtre (et sur l'icône du Dock côté
   Mac), ouverture de fichiers PDF, SVG, TIFF, PNG, JPEG. Feuille qui demande le réglage
   (`GET /api/reglages`, liste des `nom`) et, en option, client, commande et quantité.
   Envoi `POST /api/travaux` en `multipart/form-data` : champ `reglage` (nom du réglage,
   appliqué par le serveur), champs de fiche `client`, `commande`, `quantite`, puis
   `fichier` en dernier. Réponse 202 avec le travail. Barre de progression pendant l'envoi
   (les fichiers font parfois plusieurs Go).
5. **Machines** : cartes de `GET /api/machines` (nom, modèle, opérations), avec pour chacune
   ses sorties actives de `GET /api/file` (`machine` = id, `etat` `en_cours` ou `en_attente`)
   et la raison d'attente (`raison_attente`). Actions sur une sortie :
   `POST /api/file/{id}/{action}` avec `suspendre`, `reprendre`, `annuler`, `en_tete`,
   `monter`, `descendre`, `refaire`.
6. **Système** : `GET /api/systeme` (version, disque libre/total, tampon, rendus en cours
   et en attente, alertes de dossiers) et la version du client.
7. **Notifications** du système quand un travail passe à `termine` ou `echec`, et quand une
   sortie se termine.

Hors lot 1 (écrans d'administration, restent dans le client Tauri pour l'instant) :
supports, règles, balises, tarifs, tons directs, pont ERP, sauvegarde, éditeur de
machines, planche d'imbrication, détourage, mise en coupe.

À prévoir dans les clients natifs (pas dans le client Tauri) :

- **9.14** : un client relié à plusieurs serveurs RIP, avec une file unifiée.
- Réglage du travail « Lissage des images agrandies » : champ `reechantillonnage` de la
  demande (`proche_voisin` par défaut, ou `lisse`), feuille de route 9.15.

## Contrat de l'API pour les clients

- Chaque requête porte un User-Agent contenant `EKO-RIP-Client/` (sinon le serveur
  traite le poste comme un navigateur) : `EKO-RIP-Client/<version> (macOS)` ou `(Linux)`.
- Erreurs : statut 4xx/5xx, corps `{"code": "...", "detail": "..."|null}`. Afficher le
  libellé français du code s'il est connu (table `fr` de `serveur/static/index.html`,
  clés de même nom), sinon le code et le détail.
- Dates : secondes Unix (`cree_le`, `debut`, `fin`, `echeance`).
- Les champs inconnus sont ignorés : décoder avec des champs facultatifs, le serveur en
  ajoute à chaque version.

## Construction et livraison

- Mac : `clients/mac/construire.sh` sur un Mac (Xcode ou outils en ligne de commande) →
  `E-KO RIP Center.app` universelle signée ad hoc, puis `.dmg`.
- Linux : `cargo deb` dans `clients/linux/` sur le serveur de test (Ubuntu 26.04), paquet
  `e-ko-rip-center` publié sur le dépôt APT comme avant (il remplacera le client Tauri).
