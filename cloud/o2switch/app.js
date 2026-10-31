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

'use strict';

const http = require('http');
const net = require('net');
const path = require('path');
const os = require('os');
const { spawn } = require('child_process');

const PORT_INTERNE = Number(process.env.EKO_CLOUD_PORT_INTERNE || 8650);
const BINAIRE = path.join(__dirname, 'eko-rip-cloud');

let relais = null;
let arret = false;

// Un relais déjà à l'écoute (resté d'un lanceur arrêté brutalement) : on s'en sert et on
// ne relance que quand il disparaît.
function deja_lance(suite) {
  const essai = net.connect(PORT_INTERNE, '127.0.0.1');
  essai.on('connect', () => {
    essai.destroy();
    suite(true);
  });
  essai.on('error', () => suite(false));
}

function lancer() {
  if (arret) return;
  deja_lance((oui) => {
    if (oui) {
      setTimeout(lancer, 10000);
    } else {
      demarrer();
    }
  });
}

function demarrer() {
  const env = {
    ...process.env,
    EKO_CLOUD_ADDR: `127.0.0.1:${PORT_INTERNE}`,
    EKO_CLOUD_DATA: process.env.EKO_CLOUD_DATA || path.join(os.homedir(), 'eko-cloud-data'),
  };
  relais = spawn(BINAIRE, [], { env, stdio: 'inherit' });
  relais.on('exit', (code, signal) => {
    relais = null;
    if (arret) return;
    console.error(`eko-rip-cloud arrêté (${code ?? signal}), relance dans 2 s`);
    setTimeout(lancer, 2000);
  });
  relais.on('error', (e) => console.error(`eko-rip-cloud : ${e.message}`));
}

// En-têtes vers le relais : `X-Forwarded-For` posé par le frontal (LiteSpeed) est gardé
// tel quel ; sinon l'adresse vue ici.
function entetes(req) {
  const h = { ...req.headers };
  if (!h['x-forwarded-for']) h['x-forwarded-for'] = req.socket.remoteAddress || '';
  return h;
}

const serveur = http.createServer((req, res) => {
  const vers = http.request(
    { host: '127.0.0.1', port: PORT_INTERNE, method: req.method, path: req.url, headers: entetes(req) },
    (r) => {
      res.writeHead(r.statusCode, r.rawHeaders);
      r.pipe(res);
    },
  );
  vers.on('error', () => {
    if (!res.headersSent) {
      res.writeHead(502, { 'content-type': 'application/json' });
      res.end('{"code":"indisponible"}');
    } else {
      res.destroy();
    }
  });
  req.pipe(vers);
});

// WebSocket (tunnel des serveurs RIP) : la poignée de main est rejouée telle quelle vers le
// relais, puis les deux connexions sont soudées.
serveur.on('upgrade', (req, client, tete) => {
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
  client.setNoDelay(true);
  client.setKeepAlive(true, 30000);
  const fermer = () => {
    amont.destroy();
    client.destroy();
  };
  amont.on('error', fermer);
  client.on('error', fermer);
  amont.on('close', fermer);
  client.on('close', fermer);
});

// Pas de délai d'inactivité côté Node : le tunnel reste ouvert des heures.
serveur.requestTimeout = 0;
serveur.headersTimeout = 60000;
serveur.keepAliveTimeout = 65000;
serveur.timeout = 0;

for (const s of ['SIGTERM', 'SIGINT']) {
  process.on(s, () => {
    arret = true;
    if (relais) relais.kill('SIGTERM');
    serveur.close(() => process.exit(0));
    setTimeout(() => process.exit(0), 5000).unref();
  });
}

lancer();
// Passenger / lsnode fournit le port (ou la socket) dans PORT.
serveur.listen(process.env.PORT || 3000);
