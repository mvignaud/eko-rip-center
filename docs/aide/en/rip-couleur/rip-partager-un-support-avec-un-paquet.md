---
titre: Share a media with a media package
description: Export a media, its modes and the presets that use it, then import it on another server with no duplicates.
sous_rubrique: rip-couleur
position: 52
mots_cles: media package, export, import, media, modes, identifiers, another server
slug: rip-share-a-media-with-a-media-package
traduction_de: rip-partager-un-support-avec-un-paquet
---

A media package carries a media from one server to another. This article is for the administrator.

## Before you start

You are administrator of both servers. The receiving server is at least as up to date as the exporting one.

## Steps

1. On the source server, open **System** (*Système*), **Epson media** (*Supports de l'Epson*) section.
2. Click **Export** (*Exporter*) on the media's line: a file is downloaded, with this media, its modes and the presets that use it, without the ICC profile.
3. On the other server, open the same section and click **Import a media package** (*Importer un paquet support*).
4. Choose the file.
5. Add the media's ICC profile on the second server (see [Choose the colour profiles of a job](rip-choisir-les-profils-de-couleur.md)).

## Good to know

- Identifiers are kept, and a re-import creates no duplicate.
- A package made by a newer version is refused: update this server first (see [Update the server](../rip-administration/rip-mettre-a-jour-le-serveur.md)).
- Prices stay in **Ink and media costs** (*Coûts d'encre et de support*), linked by name.

## See also

- [Set up a media and its print modes](rip-regler-un-support-et-ses-modes.md)
- [Set the ink and media costs](rip-regler-les-couts-d-encre-et-de-support.md)
- [Update the server](../rip-administration/rip-mettre-a-jour-le-serveur.md)
