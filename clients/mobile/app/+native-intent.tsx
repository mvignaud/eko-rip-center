// Retour de la connexion au cloud (ekorip://oauth?code=…) : la réponse est lue par
// expo-web-browser ; le routeur ne doit pas chercher d'écran « oauth ».
export function redirectSystemPath({ path }: { path: string; initial: boolean }): string {
  return path.includes('oauth') ? '/' : path;
}
