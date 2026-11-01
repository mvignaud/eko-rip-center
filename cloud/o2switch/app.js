// Lanceur de la plate-forme E-KO RIP Cloud sur un hébergement mutualisé (o2switch, cPanel
// « Setup Node.js App »). L'hébergeur ne lance que des applications Node.js : ce fichier
// démarre le relais (binaire `eko-rip-cloud`, dans le même dossier) sur un port local et lui
// passe tout, requêtes HTTP comme WebSocket du tunnel. Aucune dépendance npm.
//
// Réglages (variables d'environnement de l'application dans cPanel) :
//   EKO_CLOUD_URL   adresse publique (https://platform.e-ko.rip)
//   EKO_CLOUD_DATA  dossier des données (défaut : ~/eko-cloud-data)
//   EKO_CLOUD_SOCKET  socket local du relais (défaut : relais.sock à côté de ce fichier).
//     Pas de port TCP : o2switch abat en quelques secondes tout programme du compte qui
//     écoute sur un port.
//   EKO_CLOUD_JETON_STATS, EKO_CLOUD_MAX_MO : transmis tels quels au relais.
//
// Journal du relais : relais.log, à côté de ce fichier (la sortie de l'application, elle,
// part dans les journaux de l'hébergeur, illisibles depuis le compte).

'use strict';

const http = require('http');
const net = require('net');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { spawn } = require('child_process');

const SOCKET = process.env.EKO_CLOUD_SOCKET || path.join(__dirname, 'relais.sock');
const BINAIRE = path.join(__dirname, 'eko-rip-cloud');
const PID = path.join(__dirname, 'relais.pid');
const JOURNAL = path.join(__dirname, 'relais.log');
// Attente maximale du relais par une requête qui arrive pendant son (re)démarrage.
const ATTENTE_MAX_MS = 15000;

let relais = null;
let arret = false;

function noter(texte) {
  try {
    fs.appendFileSync(JOURNAL, `${new Date().toISOString()} [lanceur] ${texte}\n`);
  } catch (_) {
    // Journal indisponible : rien de mieux à faire.
  }
}

// Le relais répond-il vraiment (GET /sante) ? Un port seulement ouvert ne suffit pas : un
// ancien relais en cours d'arrêt garde son écoute.
function sain(suite) {
  const req = http.get({ socketPath: SOCKET, path: '/sante', timeout: 2000 }, (r) => {
    r.resume();
    suite(r.statusCode === 200);
  });
  req.on('timeout', () => req.destroy());
  req.on('error', () => suite(false));
}

function socket_libre(suite) {
  const essai = net.connect({ path: SOCKET });
  essai.on('connect', () => {
    essai.destroy();
    suite(false);
  });
  essai.on('error', () => suite(true));
}

// Un relais resté d'un lanceur arrêté brutalement : s'il répond, on s'en sert ; s'il bloque
// le port sans répondre, on l'arrête (son pid est dans relais.pid). Le pid n'est utilisé
// que s'il désigne bien notre binaire (un pid peut être réattribué).
function ancien_pid() {
  try {
    const pid = Number(fs.readFileSync(PID, 'utf8').trim());
    if (!Number.isInteger(pid) || pid <= 1) return null;
    const ligne = fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8');
    return ligne.split('\0')[0] === BINAIRE ? pid : null;
  } catch (_) {
    return null;
  }
}

let adopte = false;
let arret_demande = 0;

function lancer() {
  if (arret || relais) return;
  sain((oui) => {
    if (oui) {
      // Relais orphelin en bonne santé : on le garde (sa sortie ne va pas dans relais.log),
      // et on revérifie régulièrement.
      if (!adopte) noter('relais déjà en marche, repris tel quel');
      adopte = true;
      setTimeout(lancer, 10000);
      return;
    }
    adopte = false;
    socket_libre((libre) => {
      if (libre) {
        arret_demande = 0;
        return demarrer();
      }
      const pid = ancien_pid();
      if (pid) {
        // D'abord SIGTERM (le relais enregistre ses données et s'arrête en 5 s au plus),
        // SIGKILL seulement s'il est encore là 6 s plus tard.
        const maintenant = Date.now();
        if (!arret_demande) arret_demande = maintenant;
        const signal = maintenant - arret_demande > 6000 ? 'SIGKILL' : 'SIGTERM';
        noter(`socket occupé par un relais qui ne répond pas (pid ${pid}) : ${signal}`);
        try {
          process.kill(pid, signal);
        } catch (_) {
          // Déjà parti.
        }
      } else {
        noter(`socket occupé par un autre programme : nouvel essai dans 2 s`);
      }
      setTimeout(lancer, 2000);
    });
  });
}

// Journal : au-delà de 5 Mo, l'ancien devient relais.log.1 (un seul gardé).
function tourner_journal() {
  try {
    if (fs.statSync(JOURNAL).size > 5 * 1024 * 1024) fs.renameSync(JOURNAL, `${JOURNAL}.1`);
  } catch (_) {
    // Pas de journal encore.
  }
}

function demarrer() {
  const env = {
    ...process.env,
    EKO_CLOUD_ADDR: `unix:${SOCKET}`,
    EKO_CLOUD_DATA: process.env.EKO_CLOUD_DATA || path.join(os.homedir(), 'eko-cloud-data'),
  };
  tourner_journal();
  let sortie = 'ignore';
  try {
    sortie = fs.openSync(JOURNAL, 'a');
  } catch (_) {
    // Sans journal, on lance quand même.
  }
  // Groupe de processus à part : quand l'hébergeur tue le groupe du lanceur (SIGKILL), le
  // relais survit et le lanceur suivant le reprend ; sur SIGTERM, le lanceur le lui transmet.
  relais = spawn(BINAIRE, [], { env, stdio: ['ignore', sortie, sortie], detached: true });
  if (typeof sortie === 'number') fs.closeSync(sortie);
  try {
    fs.writeFileSync(PID, String(relais.pid));
  } catch (_) {
    // Pas bloquant.
  }
  noter(`relais démarré (pid ${relais.pid})`);
  relais.on('exit', (code, signal) => {
    relais = null;
    noter(`relais arrêté (${code ?? signal})`);
    if (arret) return;
    setTimeout(lancer, 1000);
  });
  relais.on('error', (e) => noter(`relais : ${e.message}`));
}

// Attend que le relais accepte les connexions (au plus ATTENTE_MAX_MS).
function attendre_relais(suite, debut = Date.now()) {
  socket_libre((libre) => {
    if (!libre) return suite(true);
    if (Date.now() - debut > ATTENTE_MAX_MS) return suite(false);
    setTimeout(() => attendre_relais(suite, debut), 250);
  });
}

// En-têtes vers le relais : `X-Forwarded-For` posé par le frontal (LiteSpeed) est gardé
// tel quel ; sinon l'adresse vue ici.
function entetes(req) {
  const h = { ...req.headers };
  if (!h['x-forwarded-for']) h['x-forwarded-for'] = req.socket.remoteAddress || '';
  return h;
}

function indisponible(res) {
  if (!res.headersSent) {
    res.writeHead(503, { 'content-type': 'application/json' });
    res.end('{"code":"indisponible"}');
  } else {
    res.destroy();
  }
}

function transmettre(req, res) {
  const vers = http.request(
    { socketPath: SOCKET, method: req.method, path: req.url, headers: entetes(req) },
    (r) => {
      res.writeHead(r.statusCode, r.rawHeaders);
      r.pipe(res);
    },
  );
  vers.on('error', () => indisponible(res));
  req.pipe(vers);
}

// Le relais est vérifié avant chaque requête : pendant un redémarrage, la requête attend
// qu'il revienne au lieu de recevoir aussitôt une erreur.
const serveur = http.createServer((req, res) => {
  attendre_relais((pret) => (pret ? transmettre(req, res) : indisponible(res)));
});

// WebSocket (tunnel des serveurs RIP) : la poignée de main est rejouée telle quelle vers le
// relais, puis les deux connexions sont soudées.
serveur.on('upgrade', (req, client, tete) => {
  client.setNoDelay(true);
  client.setKeepAlive(true, 30000);
  client.on('error', () => client.destroy());
  attendre_relais((pret) => {
    if (!pret || client.destroyed) return client.destroy();
    const amont = net.connect({ path: SOCKET }, () => {
      const lignes = [`${req.method} ${req.url} HTTP/1.1`];
      for (const [nom, valeur] of Object.entries(entetes(req))) {
        for (const v of Array.isArray(valeur) ? valeur : [valeur]) lignes.push(`${nom}: ${v}`);
      }
      amont.write(lignes.join('\r\n') + '\r\n\r\n');
      if (tete && tete.length) amont.write(tete);
      // La réponse du relais est réécrite en tête canonique avant d'être soudée : le frontal
      // (LiteSpeed) ne reconnaît la montée WebSocket que sur « Connection: Upgrade » et
      // « Upgrade: websocket » écrits ainsi ; sinon il traite la suite comme un corps de
      // réponse ordinaire (découpé en blocs) et les trames arrivent abîmées.
      lire_tete(amont, (entete, reste) => {
        client.write(entete);
        if (reste.length) client.write(reste);
        amont.pipe(client);
        client.pipe(amont);
      });
    });
    amont.setNoDelay(true);
    const fermer = () => {
      amont.destroy();
      client.destroy();
    };
    amont.on('error', fermer);
    amont.on('close', fermer);
    client.on('close', fermer);
  });
});

// Lit la tête de réponse HTTP du relais (jusqu'à la ligne vide) et la rend sous forme
// canonique : ligne d'état, puis noms d'en-têtes en Casse-Habituelle, et « Upgrade » comme
// valeur de Connection pour une réponse 101. `suite(tete, reste)` reçoit aussi les octets
// déjà lus après la tête (début du flux WebSocket).
const NOMS = {
  connection: 'Connection',
  upgrade: 'Upgrade',
  'sec-websocket-accept': 'Sec-WebSocket-Accept',
  'sec-websocket-protocol': 'Sec-WebSocket-Protocol',
  'content-type': 'Content-Type',
  'content-length': 'Content-Length',
};
function lire_tete(amont, suite) {
  let tampon = Buffer.alloc(0);
  const lire = (morceau) => {
    tampon = Buffer.concat([tampon, morceau]);
    const fin = tampon.indexOf('\r\n\r\n');
    if (fin < 0) {
      if (tampon.length > 16384) amont.destroy();
      return;
    }
    amont.removeListener('data', lire);
    amont.pause();
    const lignes = tampon.subarray(0, fin).toString('latin1').split('\r\n');
    const etat = lignes.shift();
    const montee = / 101 /.test(`${etat} `);
    const sortie = [montee ? 'HTTP/1.1 101 Switching Protocols' : etat];
    for (const ligne of lignes) {
      const i = ligne.indexOf(':');
      if (i <= 0) continue;
      const nom = ligne.slice(0, i).trim().toLowerCase();
      let valeur = ligne.slice(i + 1).trim();
      if (montee && nom === 'connection') valeur = 'Upgrade';
      if (montee && nom === 'upgrade' && valeur.toLowerCase() === 'websocket') valeur = 'websocket';
      if (montee && (nom === 'content-length' || nom === 'transfer-encoding' || nom === 'date')) continue;
      sortie.push(`${NOMS[nom] || nom}: ${valeur}`);
    }
    suite(Buffer.from(sortie.join('\r\n') + '\r\n\r\n', 'latin1'), tampon.subarray(fin + 4));
  };
  amont.on('data', lire);
}

// Pas de délai d'inactivité côté Node : le tunnel reste ouvert des heures.
serveur.requestTimeout = 0;
serveur.headersTimeout = 60000;
serveur.keepAliveTimeout = 65000;
serveur.timeout = 0;

for (const s of ['SIGTERM', 'SIGINT']) {
  process.on(s, () => {
    arret = true;
    noter(`lanceur arrêté (${s})`);
    if (relais) relais.kill('SIGTERM');
    serveur.close(() => process.exit(0));
    setTimeout(() => process.exit(0), 5000).unref();
  });
}

lancer();
// Passenger / lsnode fournit le port (ou la socket) dans PORT.
serveur.listen(process.env.PORT || 3000);
