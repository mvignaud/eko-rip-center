# Graphtec FC7000

## Ce que fait le RIP

Il envoie la découpe en HP-GL, au pas de 0,025 mm.

## Brancher

Réseau (IP fixe) en connexion **Réseau (TCP)**, ou série / USB-série.

## Sur la Graphtec

Commandes en **HP-GL** (Menu > Interface > Command) ; le repérage
**ARMS** se règle au panneau de la machine.

## Dans le RIP

Pas de **0,025 mm** ; réglages de coupe ; **Décalage X / Y**.

## Vérifier

`nc -vz <ip> <port>` depuis le serveur.

## Ennuis courants

- *Rien ne coupe* : l'interface n'est pas en HP-GL.
- *Sortie en échec, machine injoignable* : vérifier l'adresse, le port et le câble.

Voir : [découpe](../utilisation.md#découpe), [machines](../utilisation.md#machines).
