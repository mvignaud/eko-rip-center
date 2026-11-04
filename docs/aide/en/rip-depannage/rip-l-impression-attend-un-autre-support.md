---
titre: Fix a print waiting on "Other media" or showing "Too wide"
description: Fix an output that is waiting because the loaded media is not the job's media or because the page exceeds the roll width.
sous_rubrique: rip-depannage
position: 41
mots_cles: other media, too wide, loaded roll, roll width, loaded media, epson
slug: rip-fix-print-waiting-for-other-media
traduction_de: rip-l-impression-attend-un-autre-support
---

An output stays in the queue with a red "Autre support" ("Other media") or "Trop large" ("Too wide") badge. This article gives the steps to follow, for operators.

## Before you start

You can see the output in the queue in the **Production** view. To read the media, the Epson must have an SNMP address (see [Read the media loaded in the Epson and its real consumption](../rip-couleur/rip-lire-le-media-charge-dans-l-epson.md)).

## Steps

1. Hover over the output's red badge to read the details.
2. For **Other media** (*Autre support*): load the job's media in the Epson. The output starts by itself.
3. If the media is already the right one but is misrecognised, open **System** (*Système*) > **Media loaded in the Epson** (*Média chargé dans l'Epson*) and choose the media by hand from the list (your choice takes priority until you go back to **— lu sur l'imprimante —**, "read from the printer").
4. For **Too wide** (*Trop large*): the page, including marks, exceeds the width of the machine or of the loaded roll. Load a wider roll, or reduce the page (see [Crop, resize or rotate a page](../rip-travaux/rip-recadrer-redimensionner-ou-tourner-une-page.md)).
5. If needed, click **Resume** (*Reprendre*) or **Redo** (*Refaire*) on the output.

## Good to know

- A media number unknown to the catalogue is flagged: declare it (see [Set up a media and its print modes](../rip-couleur/rip-regler-un-support-et-ses-modes.md)).
- If the loaded media is unknown, nothing is blocked.
- A page can be rotated automatically when it exceeds the roll width but fits when rotated by 90° (**Rotate automatically if the artwork exceeds the roll width** box, *Tourner seul si le visuel dépasse la laize*).

## See also

- [Understand why an output is waiting](../rip-travaux/rip-comprendre-les-raisons-d-attente-d-une-sortie.md)
- [Manage a machine's queue](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
- [Connect the Epson SureColor SC-S80600](../rip-machines/rip-brancher-epson-sc-s80600.md)
