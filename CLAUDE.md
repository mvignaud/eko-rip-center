# E-KO RIP Center — consignes pour Claude

## Release : publier sur le dépôt APT (obligatoire)

Chaque release se termine par la publication des paquets Linux sur le dépôt APT
`https://apt.e-ko.fr/rip/`. C'est par lui que les postes Linux et les serveurs RIP se mettent
à jour (`sudo apt update && sudo apt upgrade`). Une release qui n'est pas sur le dépôt n'arrive
chez personne.

Sur le serveur de test (`mathieu@eko-rip`, 192.168.10.109), une fois les `.deb` construits :

```bash
~/eko-depot/outils/publier-depot.sh \
  eko-rip-serveur_<version>-1_amd64.deb \
  "E-KO RIP Center_<version>_amd64.deb"
```

- **Les deux paquets à chaque release** : le serveur (`eko-rip-serveur`, via cargo-deb) et le
  client Linux (`e-ko-rip-center`, via `npx @tauri-apps/cli@2 build --bundles deb` dans
  `client/`). La CI ne construit plus le client Linux, il se construit sur le serveur de test.
- Le script signe avec la clé GPG `depot@e-ko.fr` du trousseau du serveur de test et envoie par
  rsync au VPS OVH (`Host apt.e-ko.fr` dans `~/.ssh/config`, clé `~/.ssh/eko_depot`, compte
  `depot` limité à `/srv/depot`). Ne jamais régénérer cette clé : les postes refuseraient les
  mises à jour.
- `~/eko-depot/outils/` est une copie de `outils/publier-depot.sh` et `outils/installer-depot.sh`.
  Quand on les modifie dans le dépôt, il faut aussi recopier la nouvelle version là-bas.
- Vérification : `curl -s https://apt.e-ko.fr/rip/Packages | grep -A1 '^Package:'` doit lister
  les nouvelles versions.
- Le VPS est décrit dans eko-infra (`ansible/depot.yml`).
