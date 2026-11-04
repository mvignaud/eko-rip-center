---
titre: Act on an output from the mobile app
description: Start, pause, stop, rerun or move an output to the front from the app's Queue tab.
sous_rubrique: rip-travaux
position: 71
mots_cles: mobile app, pause, stop, rerun, move to front, resume, output
slug: rip-act-on-an-output-from-the-mobile-app
traduction_de: rip-agir-sur-une-sortie-depuis-l-application-mobile
---

From the **Queue** (*File*) tab, you can act on an output without walking over to the workstation.

## Before you start

The app is connected with at least an **Operator** (*Opérateur*) account. The output has not yet gone to the machine, except for **Rerun**.

## Steps

1. Open the **Queue** tab.
2. Tap the output concerned.
3. Choose the action you want from the list.
4. Confirm if the app asks you to.

## Good to know

The actions offered depend on the state of the output:

| Output state | Possible actions |
|---|---|
| Waiting | **Pause**, **Move to front** (*Passer en tête*), **Stop** |
| Held | **Start** (*Lancer*) or **Resume** (*Reprendre*), **Move to front**, **Stop** |
| Finished or failed | **Rerun** (*Relancer*; creates a new identical output) |
| Nested in a sheet | No action |

- **Pause** and **Stop** only act on an output that has not yet gone to the machine. **Stop** cancels a waiting or held output.
- A print already sent to the machine cannot be stopped from the app: the server cannot interrupt it yet. Use the machine itself (see [Printing continues after Stop](../rip-depannage/rip-impression-continue-apres-stop.md)).

## See also

- [Follow the print queue on the mobile app](rip-suivre-la-file-sur-l-application-mobile.md)
- [Manage a machine's queue](rip-gerer-la-file-d-une-machine.md)
- [Follow jobs and start printing](rip-suivre-les-travaux.md)
