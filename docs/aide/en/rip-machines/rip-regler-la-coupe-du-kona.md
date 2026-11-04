---
titre: Set up the Kona cut
description: Declare the cut settings (speed, force, acceleration, tool, pattern), get the .plt file as the Kona receives it and flip the cut.
sous_rubrique: rip-machines
position: 47
mots_cles: cut settings, speed, force, acceleration, tool, pattern, spreading, plt, flipped cut, guillotine
slug: rip-set-up-kona-cut
traduction_de: rip-regler-la-coupe-du-kona
---

As in Mutoh CutServer, each cut layer goes out with its cut setting. This article is for the administrator.

## Before you start

The Kona is connected (see [Connect the Mutoh Kona 1650](rip-brancher-mutoh-kona-1650.md)). You need an administrator account.

## Steps

1. Open **Administration…**, the **System** view (*Système*), the **Machines** section, the Kona's page.
2. Click **Add a setting** (*Ajouter un réglage*) (under **Cut settings**, *Réglages de coupe*) and give it a name (the name of a cut layer, to apply it to that layer alone).
3. Enter the **Speed (cm/s)** (*Vitesse (cm/s)*), the **Force (g)** and the **Acceleration (G)** (*Accélération (G)*).
4. If needed, tick **Guillotine values** (*Valeurs du massicot*) (nothing is sent: the Kona keeps its own values), force a **Tool** (*Outil*) (1 to 4) or leave it on **default** (*par défaut*).
5. For a cutting pattern, enter the **Pattern (down/up, mm)** (*Motif (bas/haut, mm)*): up to 10 "down/up" pairs in mm (`8/2 8/2` = cut 8 mm, lift 2 mm), and tick **Pattern spreading** (*Diffusion du motif*) for a whole number of patterns per straight line (the cut starts and ends at sharp corners).
6. Click **Save** (*Enregistrer*). The first setting is the default.

## Good to know

- To get the file as the Kona receives it, to send by hand: **.plt file** (*Fichier .plt*) with `?machine=kona` (serial port 9600, 8N1, RTS/CTS).
- If the cut comes out rotated by 180°, tick **Flipped cut (180°)** (*Découpe retournée (180°)*) on the machine.
- With a frame, the RIP declares the two opposite corners of the frame (middles of the lines), the Kona scans it, reads the code, then cuts.
- Each cut layer goes out with its own setting (see [Order the cut layers](../rip-travaux/rip-ordonner-les-couches-de-decoupe.md)).
- The Kona must be set to a resolution of **0.010 mm**; the machine's cutting step is 0.01 mm.

## See also

- [Choose the Kona registration](rip-choisir-le-reperage-kona.md)
- [Order the cut layers](../rip-travaux/rip-ordonner-les-couches-de-decoupe.md)
- [Fix a cut that is offset from the print](../rip-depannage/rip-decoupe-decalee.md)
