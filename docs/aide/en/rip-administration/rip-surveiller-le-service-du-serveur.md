---
titre: Monitor the server service
description: Check that the server is running, read its log and restart it without needlessly losing work.
sous_rubrique: rip-administration
position: 20
mots_cles: service, systemctl, journalctl, restart, log, status, system, monitoring
slug: rip-monitor-the-server-service
traduction_de: rip-surveiller-le-service-du-serveur
---

The server is a Linux service named `eko-rip-serveur`. This article gives the three everyday commands and what a restart changes for jobs in progress.

## Before you start

You have a terminal on the server (or SSH access) and `sudo` access. Warn the operators before a restart: it interrupts the render in progress.

## Steps

1. Check that the service is running: `systemctl status eko-rip-serveur`.
2. Follow the log live: `journalctl -u eko-rip-serveur -f`. It records every failed render and every failed output, with the machine, job, page and reason, as well as the "aucun utilisateur déclaré" (no user declared) warning as long as access is open.
3. Restart if needed: `sudo systemctl restart eko-rip-serveur`.
4. In the client, open **Administration…** (*Administration…*) then the **System** (*Système*) view: it shows the version, the job retention period, the free space on the disk, the spool and the renders in progress.

   ![System view with the version, retention period, free space, spool and simultaneous renders](../../images/rip-systeme-version.png)
5. After a restart, use **Retry** (*Relancer*) on the jobs that ended in "interrompu" (interrupted) failure, and ask users to sign in again.

## Good to know

- A restart interrupts the render in progress: the job ends in "interrompu" failure and is restarted with **Retry**. It does not restart by itself.
- Queued outputs stay in the queue; sessions are lost, so everyone signs in again. A restart also lifts the 5-minute block on an account name after five wrong passwords.
- The service restarts by itself with the PC.
- If the service does not start, read the end of the log: `journalctl -u eko-rip-serveur -e`.

## See also

- [Change the server's advanced settings](rip-modifier-les-reglages-avances-du-serveur.md)
- [Update the server](rip-mettre-a-jour-le-serveur.md)
- [Fix a failed render or a rejected file](../rip-depannage/rip-un-rendu-echoue.md)
- [Recover after a power cut](rip-reprendre-apres-une-coupure-de-courant.md)
