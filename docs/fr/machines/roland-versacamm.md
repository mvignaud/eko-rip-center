# Roland VersaCAMM VP-300 / VP-540

## Ce que fait le RIP

La Roland VersaCAMM est une imprimante-découpeuse. Le RIP lui envoie la
découpe par le réseau (laize 762 ou 1371 mm). L'impression par la Roland n'est pas encore
pilotée par le RIP.

> [!NOTE]
> La prise en charge de la Roland est récente : signaler tout écart avec **Signaler un
> problème…** dans le client.

## Brancher

Réseau, IP fixe, connexion **Réseau (TCP)**.

## Vérifier

`nc -vz <ip> <port>` depuis le serveur.
