# Mutoh Kona 1650

## Ce que fait le RIP

Il envoie la découpe comme Mutoh CutServer (HP-GL au pas de
0,01 mm), avec lecture du cadre et du code-barres pour le repérage.

## Brancher

USB (adaptateur série FTDI). La règle udev du paquet crée `/dev/kona` pour le
groupe `dialout`. Connexion **Série / USB-série**, port `/dev/kona`, réglages
`9600 cs8 -cstopb -parenb crtscts raw -echo` (9600 bauds, 8N1, RTS/CTS).

## Sur le Kona

Résolution **0,010 mm**.

## Dans le RIP

Pas de découpe **0,01 mm** ; **réglages de coupe** (vitesse, force,
accélération, outil, motif), le premier servant par défaut ; **Décalage X / Y** mesuré sur
une coupe d'essai ; **Découpe retournée** si la coupe sort tournée de 180°.

## Repérage

À l'envoi, **Repérage Kona** ; le **cadre + code-barres (type 2)** est celui
éprouvé à l'atelier. Engager dans le Kona le bord que montre la flèche ▲ du cadre.

## Vérifier

`ls -l /dev/kona` après l'avoir branché.

## Ennuis courants

- *`/dev/kona` absent* : débrancher, rebrancher, puis `journalctl -k | tail`.
- *Arrêt sur « EPOS 6213 »* : cadre type 1 sans code-barres envoyé en direct ; le Kona ne
  le lit pas seul. Prendre le type 2.
- *Découpe décalée* : coupe d'essai, mesurer, saisir le **Décalage X / Y**.

Voir : [repérage pour le Kona](../utilisation.md#repérage-pour-le-kona),
[découpe](../utilisation.md#découpe), [variantes](../utilisation.md#variantes-de-machine).
