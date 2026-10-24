// Clés en double au même niveau d'un littéral d'objet (la dernière écrase l'autre sans erreur).
const src = require("fs").readFileSync(process.argv[2], "utf8");
const debut = src.indexOf("  fr: {");
let i = src.lastIndexOf("{", debut - 1), pile = [new Map()], chemin = [""], dans = null, n = 0;
for (; i < src.length; i++) {
  const c = src[i];
  if (dans) { if (c === "\\") { i++; continue; } if (c === dans) dans = null; continue; }
  if (c === '"' || c === "'" || c === "`") { dans = c; continue; }
  if (c === "{") { const m = /([A-Za-z_]\w*)\s*:\s*$/.exec(src.slice(Math.max(0, i - 60), i)); pile.push(new Map()); chemin.push(m ? m[1] : "?"); continue; }
  if (c === "}") { pile.pop(); chemin.pop(); if (!pile.length) break; continue; }
  if (pile.length >= 2) {
    const m = /^([A-Za-z_]\w*)\s*:/.exec(src.slice(i, i + 50));
    const avant = src[i - 1];
    if (m && /[\s,{]/.test(avant)) { const k = m[1], t = pile[pile.length - 1]; if (t.has(k)) { console.log(`doublon : ${chemin.join(".")}.${k}`); n++; } t.set(k, 1); i += m[0].length - 1; }
  }
}
process.exit(n ? 1 : 0);
