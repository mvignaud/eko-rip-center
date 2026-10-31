# E-KO RIP — application mobile

Application de suivi (iOS / Android) du RIP E-KO : file d'attente, machines (encres, rouleau, alertes),
actions sur une sortie (lancer / reprendre, pause, stop, relancer, passer en tête) et
**notifications push** des erreurs machine. Pas d'envoi de fichiers ni de réglage d'impression ou de découpe.

Expo SDK 58, React Native, TypeScript, expo-router (onglets). Aucune bibliothèque d'UI ni d'état.

## Structure

- `app/` : écrans (`connexion`, onglets File / Machines / Alertes / Réglages).
- `src/api.ts` : client HTTP, types, authentification (interface `Auth`, implémentation cookie de session).
- `src/session.tsx` : contexte de session (connexion, reprise, déconnexion, notifications).
- `src/notifications.ts` : jeton push Expo, canal Android « alertes », enregistrement sur le serveur.
- `src/alertes.tsx` : événements et pastille de l'onglet Alertes.

L'adresse du serveur peut contenir un chemin (`https://rip.e-ko.fr/s/atelier`) : les URL sont construites
par simple concaténation `base + /api/...`.

## Lancer en développement

```bash
cd clients/mobile
npm install
npx expo start
```

Limites d'Expo Go : depuis le SDK 53, les **notifications push ne fonctionnent plus dans Expo Go sur Android**
(et il faut un projet EAS pour iOS). Pour tester les notifications, utiliser un **development build** :

```bash
eas build -p android --profile development   # ou -p ios
npx expo start --dev-client
```

Les notifications ne marchent que sur un appareil physique (pas sur simulateur).
Sans projet EAS, l'app fonctionne mais Réglages affiche « Notifications : projet EAS non configuré ».

## Initialiser EAS

```bash
npm install -g eas-cli
eas login
eas init        # remplace "A_RENSEIGNER_PAR_EAS_INIT" dans app.json (extra.eas.projectId)
```

Vérifier ensuite que `extra.eas.projectId` contient bien l'identifiant du projet.

## Builds

```bash
eas build -p ios --profile preview        # distribution interne (ad hoc, appareils enregistrés)
eas build -p android --profile preview    # APK interne
eas build -p ios --profile production
eas build -p android --profile production
```

Profils (`eas.json`) : `development` (dev client), `preview` (distribution interne), `production` (stores).

## Identifiants push

```bash
eas credentials
```

- **iOS** : clé APNs (.p8) liée au compte Apple Developer (EAS peut la générer).
- **Android** : compte de service **FCM V1** (Firebase) envoyé à EAS, et fichier `google-services.json`
  du projet Firebase placé à la racine de `clients/mobile/`, puis référencé dans `app.json` :
  `"android": { "googleServicesFile": "./google-services.json" }`.
  Le package Android est `fr.eko.rip.mobile`. Le fichier ne doit pas être ignoré par git si le build part de EAS
  depuis le dépôt (ou le fournir comme fichier secret EAS).

## Publication sur les stores

```bash
eas submit -p ios        # App Store Connect (TestFlight)
eas submit -p android    # Google Play (compte de service Play requis)
```

## Réseau

- L'app parle en HTTP clair au serveur (`http://192.168.x.x:8640`) : ATS (iOS) et `usesCleartextTraffic` (Android)
  sont autorisés dans la configuration. Il faut être sur le réseau de l'atelier ou le VPN.
- Chaque requête envoie `User-Agent: EKO-RIP-Client/mobile-<version>` (exigé par le serveur).
- **Le serveur RIP doit pouvoir joindre `https://exp.host` en sortie** : c'est par ce service Expo qu'il envoie
  les notifications push (APNs / FCM).

## Vérifications

```bash
npx tsc --noEmit
npx expo export --platform ios --platform android --output-dir /tmp/mobile-export-check
npx expo-doctor
```
