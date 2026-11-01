// Lanceur de la plate-forme E-KO RIP Cloud sur un hébergement mutualisé (o2switch, cPanel
// « Setup Node.js App »). L'hébergeur ne lance que des applications Node.js : ce fichier
// démarre le relais (binaire `eko-rip-cloud`, dans le même dossier) sur un port local et lui
// passe tout, requêtes HTTP comme WebSocket du tunnel. Aucune dépendance npm.
//
// Réglages (variables d'environnement de l'application dans cPanel) :
//   EKO_CLOUD_URL   adresse publique (https://platform.e-ko.rip)
//   EKO_CLOUD_DATA  dossier des données (défaut : ~/eko-cloud-data)
//   EKO_CLOUD_PORT_INTERNE  port local du relais (défaut : 8650)
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

const PORT_INTERNE = Number(process.env.EKO_CLOUD_PORT_INTERNE || 8650);
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
  const req = http.get({ host: '127.0.0.1', port: PORT_INTERNE, path: '/sante', timeout: 2000 }, (r) => {
    r.resume();
    suite(r.statusCode === 200);
  });
  req.on('timeout', () => req.destroy());
  req.on('error', () => suite(false));
}

function port_libre(suite) {
  const essai = net.connect(PORT_INTERNE, '127.0.0.1');
  essai.on('connect', () => {
    essai.destroy();
    suite(false);
  });
  essai.on('error', () => suite(true));
}

// Un relais resté d'un lanceur arrêté brutalement : s'il répond, on s'en sert ; s'il bloque
// le port sans répondre, on l'arrête (son pid est dans relais.pid).
function ancien_pid() {
  try {
    const pid = Number(fs.readFileSync(PID, 'utf8').trim());
    return Number.isInteger(pid) && pid > 1 ? pid : null;
  } catch (_) {
    return null;
  }
}

function lancer() {
  if (arret || relais) return;
  sain((oui) => {
    if (oui) {
      // Relais orphelin en bonne santé : on le garde, et on revérifie régulièrement.
      setTimeout(lancer, 10000);
      return;
    }
    port_libre((libre) => {
      if (libre) return demarrer();
      const pid = ancien_pid();
      if (pid) {
        noter(`port ${PORT_INTERNE} occupé par un relais qui ne répond pas (pid ${pid}) : arrêt forcé`);
        try {
          process.kill(pid, 'SIGKILL');
        } catch (_) {
          // Déjà parti.
        }
      } else {
        noter(`port ${PORT_INTERNE} occupé par un autre programme : nouvel essai dans 2 s`);
      }
      setTimeout(lancer, 2000);
    });
  });
}

function demarrer() {
  const env = {
    ...process.env,
    EKO_CLOUD_ADDR: `127.0.0.1:${PORT_INTERNE}`,
    EKO_CLOUD_DATA: process.env.EKO_CLOUD_DATA || path.join(os.homedir(), 'eko-cloud-data'),
  };
  let sortie = 'ignore';
  try {
    sortie = fs.openSync(JOURNAL, 'a');
  } catch (_) {
    // Sans journal, on lance quand même.
  }
  relais = spawn(BINAIRE, [], { env, stdio: ['ignore', sortie, sortie] });
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
  port_libre((libre) => {
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
    { host: '127.0.0.1', port: PORT_INTERNE, method: req.method, path: req.url, headers: entetes(req) },
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
    const amont = net.connect(PORT_INTERNE, '127.0.0.1', () => {
      const lignes = [`${req.method} ${req.url} HTTP/1.1`];
      for (const [nom, valeur] of Object.entries(entetes(req))) {
        for (const v of Array.isArray(valeur) ? valeur : [valeur]) lignes.push(`${nom}: ${v}`);
      }
      amont.write(lignes.join('\r\n') + '\r\n\r\n');
      if (tete && tete.length) amont.write(tete);
      amont.pipe(client);
      client.pipe(amont);
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
