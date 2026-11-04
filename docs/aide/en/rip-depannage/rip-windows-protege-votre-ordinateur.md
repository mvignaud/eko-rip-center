---
titre: Handle the "Windows protected your PC" warning
description: Allow the E-KO RIP Center client installer once; it is not yet signed for Windows.
sous_rubrique: rip-depannage
position: 74
mots_cles: windows, smartscreen, warning, installer, unsigned
slug: rip-handle-windows-protected-your-pc
traduction_de: rip-windows-protege-votre-ordinateur
---

When you open the Windows installer, a blue "Windows a protégé votre ordinateur" ("Windows protected your PC") screen blocks the launch. The Windows client is not yet signed: one authorisation is enough, once per workstation.

## Before you start

You downloaded the installer from the official page https://e-ko.rip/telecharger/#windows. You can compare its SHA-256 fingerprint with the one shown on the page.

## Steps

1. On the warning screen, click **More info** (*Informations complémentaires*).
2. Click **Run anyway** (*Exécuter quand même*).
3. Follow the installation wizard.

## Good to know

- The Mac client, on the other hand, is signed and verified by Apple: it opens without a warning.
- The client installs for the current user, without administrator rights.

## See also

- [Install the client on Windows](../rip-premiers-pas/rip-installer-le-client-windows.md)
- [Install the client on Mac](../rip-premiers-pas/rip-installer-le-client-mac.md)
- [The client cannot find the server](rip-client-ne-trouve-pas-le-serveur.md)
