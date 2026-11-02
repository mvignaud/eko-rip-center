namespace EkoRip.Coeur;

public enum Rubrique { Travaux, Machines, Systeme }

public enum Phase
{
    /// <summary>Au lancement, avant d'avoir interrogé le serveur.</summary>
    Demarrage,
    /// <summary>Il faut une adresse de serveur (première fois, serveur injoignable, changement).</summary>
    Adresse,
    /// <summary>Le serveur demande un identifiant.</summary>
    Connexion,
    Connecte,
}

public sealed record MessageUtilisateur(string Titre, string Texte);

/// <summary>Un fichier en cours d'envoi, avec sa progression.</summary>
public sealed class EnvoiEnCours
{
    public Guid Id { get; } = Guid.NewGuid();
    public string Nom { get; }
    public double Progression { get; set; }
    public bool Termine { get; set; }
    public string? Erreur { get; set; }
    internal CancellationTokenSource? Annulation { get; set; }

    public EnvoiEnCours(string nom) => Nom = nom;

    public bool EnCours => !Termine && Erreur is null;
}

/// <summary>
/// État de l'application : connexion, données du serveur, actions. Sans aucune dépendance à
/// l'interface (reprise d'<c>Etat.swift</c>). À utiliser depuis le fil de l'interface : les
/// continuations d'<c>await</c> y reviennent seules, et <see cref="Changed"/> est levé sur ce fil.
/// L'interface relit alors les propriétés et se redessine.
/// </summary>
public sealed class Controleur : IDisposable
{
    private readonly Parametres _parametres;
    private readonly Action<Parametres> _enregistrer;
    private readonly Func<string, Api> _fabriqueApi;
    private readonly TimeSpan _intervalle;
    private readonly SemaphoreSlim _verrou = new(1, 1);
    private readonly SuiviNotifications _suivi = new();
    private CancellationTokenSource? _boucle;
    private CancellationTokenSource? _annonce;
    private int _cycle;

    public Controleur(Parametres parametres, Action<Parametres> enregistrer, Func<string, Api>? fabriqueApi = null, TimeSpan? intervalle = null)
    {
        _parametres = parametres;
        _enregistrer = enregistrer;
        _fabriqueApi = fabriqueApi ?? (racine => new Api(racine));
        _intervalle = intervalle ?? TimeSpan.FromSeconds(3);
    }

    // ------------------------------------------------------------ Événements

    /// <summary>Une donnée a changé : l'interface se redessine.</summary>
    public event Action? Changed;

    /// <summary>Un travail ou une sortie vient de se terminer : notification du système.</summary>
    public event Action<NotificationRip>? NotificationAEnvoyer;

    /// <summary>Des fichiers acceptés ont été déposés ou choisis : l'interface ouvre la feuille d'envoi.</summary>
    public event Action<IReadOnlyList<string>>? FichiersProposes;

    private void Signaler() => Changed?.Invoke();

    // ------------------------------------------------------------ État

    public Phase Phase { get; private set; } = Phase.Demarrage;
    public bool Occupe { get; private set; }
    public string? ErreurConnexion { get; private set; }
    public InfoSession? Session { get; private set; }
    /// <summary>Adresse affichable (« hôte:8640 »).</summary>
    public string Adresse { get; private set; } = "";
    public Api? Api { get; private set; }

    public IReadOnlyList<Travail> Travaux { get; private set; } = Array.Empty<Travail>();
    public IReadOnlyList<Sortie> Sorties { get; private set; } = Array.Empty<Sortie>();
    public IReadOnlyList<Machine> Machines { get; private set; } = Array.Empty<Machine>();
    public IReadOnlyList<Reglage> Reglages { get; private set; } = Array.Empty<Reglage>();
    public InfoSysteme? Systeme { get; private set; }
    /// <summary>Dernière erreur d'actualisation (serveur injoignable…), effacée au prochain succès.</summary>
    public string? ServeurHorsLigne { get; private set; }

    public Rubrique Rubrique { get; private set; } = Rubrique.Travaux;
    public string? Selection { get; private set; }
    /// <summary>Page affichée dans l'inspecteur, à partir de 1.</summary>
    public int Page { get; private set; } = 1;
    public string Recherche { get; private set; } = "";
    public ColonneTri ColonneTri { get; private set; } = ColonneTri.Date;
    public bool TriDecroissant { get; private set; } = true;
    public MessageUtilisateur? Message { get; private set; }
    public string? Annonce { get; private set; }
    public IReadOnlyList<EnvoiEnCours> Envois => _envois;
    private readonly List<EnvoiEnCours> _envois = new();

    public Parametres Parametres => _parametres;

    // ------------------------------------------------------------ Sélection et filtres

    public Travail? TravailSelectionne => Selection is null ? null : Travaux.FirstOrDefault(t => t.Id == Selection);

    /// <summary>Travaux filtrés par la recherche puis triés selon la colonne choisie.</summary>
    public IReadOnlyList<Travail> TravauxAffiches =>
        Regles.Trier(Regles.Filtrer(Travaux, Recherche), ColonneTri, TriDecroissant);

    public IReadOnlyList<Sortie> SortiesDe(Travail t) => Regles.SortiesDe(Sorties, t);
    public IReadOnlyList<Sortie> SortiesActivesPour(Machine m) => Regles.SortiesActivesPour(Sorties, m);
    public IReadOnlyList<Machine> MachinesPour(Travail t) => Regles.MachinesPour(Machines, t);
    public string NomMachine(string id) => Regles.NomMachine(Machines, id);
    public bool PeutSuspendre(Travail t) => Regles.PeutSuspendre(Sorties, t);
    public bool PeutReprendre(Travail t) => Regles.PeutReprendre(Sorties, t);
    public int PageValide(Travail t) => Regles.PageValide(t, Page);

    public void ChoisirRecherche(string texte)
    {
        if (texte == Recherche) return;
        Recherche = texte;
        Signaler();
    }

    /// <summary>Clic sur un en-tête de colonne : même colonne, sens inversé ; autre colonne, croissant.</summary>
    public void Trier(ColonneTri colonne)
    {
        if (colonne == ColonneTri) TriDecroissant = !TriDecroissant;
        else { ColonneTri = colonne; TriDecroissant = false; }
        Signaler();
    }

    public void Selectionner(string? id)
    {
        if (id == Selection) return;
        Selection = id;
        Page = 1;
        Signaler();
    }

    public void ChoisirPage(int page)
    {
        var t = TravailSelectionne;
        var valide = t is null ? 1 : Regles.PageValide(t, page);
        if (valide == Page) return;
        Page = valide;
        Signaler();
    }

    public async Task ChoisirRubrique(Rubrique r)
    {
        if (r == Rubrique) return;
        Rubrique = r;
        Signaler();
        await Rafraichir();
    }

    // ------------------------------------------------------------ Connexion

    /// <summary>Au lancement : reprend l'adresse retenue, ou demande une adresse.</summary>
    public async Task Demarrer()
    {
        if (Phase != Phase.Demarrage) return;
        var retenue = _parametres.Adresse;
        if (string.IsNullOrEmpty(retenue))
        {
            Phase = Phase.Adresse;
            Signaler();
        }
        else
        {
            await Connecter(retenue);
        }
    }

    /// <summary>Interroge <c>GET /api/session</c> ; puis le tableau de bord, ou l'écran d'identifiant.</summary>
    public async Task Connecter(string saisie)
    {
        var racine = Api.Normaliser(saisie);
        if (racine is null)
        {
            ErreurConnexion = Libelles.Erreur("adresse_invalide", null);
            Phase = Phase.Adresse;
            Signaler();
            return;
        }
        ArreterBoucle();
        Vider();
        Api?.Dispose();
        var nouvelle = _fabriqueApi(racine);
        Api = nouvelle;
        Adresse = Api.Affichable(racine);
        ErreurConnexion = null;
        Occupe = true;
        Signaler();
        try
        {
            var s = await nouvelle.LireSession();
            if (Api != nouvelle) return;
            _parametres.Adresse = Adresse;
            _enregistrer(_parametres);
            Session = s;
            if (s.AccesLibre == true || s.Utilisateur is not null)
                await Entrer();
            else
                Phase = Phase.Connexion;
        }
        catch (Exception e) when (e is not OperationCanceledException)
        {
            if (Api != nouvelle) return;
            ErreurConnexion = Texte(e);
            Phase = Phase.Adresse;
        }
        finally
        {
            Occupe = false;
            Signaler();
        }
    }

    public async Task OuvrirSession(string nom, string motDePasse)
    {
        var api = Api;
        if (api is null) return;
        ErreurConnexion = null;
        Occupe = true;
        Signaler();
        try
        {
            await api.OuvrirSession(nom, motDePasse);
            _parametres.DernierUtilisateur = nom;
            _enregistrer(_parametres);
            Session = await api.LireSession();
            await Entrer();
        }
        catch (Exception e) when (e is not OperationCanceledException)
        {
            ErreurConnexion = Texte(e);
        }
        finally
        {
            Occupe = false;
            Signaler();
        }
    }

    public async Task Deconnecter()
    {
        ArreterBoucle();
        var api = Api;
        if (api is not null)
        {
            try { await api.FermerSession(); }
            catch (Exception e) when (e is ErreurApi) { /* déconnecté de toute façon */ }
        }
        Vider();
        Session = null;
        ErreurConnexion = null;
        Phase = Phase.Connexion;
        Signaler();
    }

    /// <summary>Menu « Changer de serveur… » : retour à l'écran d'adresse.</summary>
    public void ChangerServeur()
    {
        ArreterBoucle();
        Vider();
        ErreurConnexion = null;
        Phase = Phase.Adresse;
        Signaler();
    }

    private async Task Entrer()
    {
        Phase = Phase.Connecte;
        Signaler();
        await ChargerReference();
        await Rafraichir();
        LancerBoucle();
    }

    private void RetourConnexion()
    {
        if (Phase != Phase.Connecte) return;
        ArreterBoucle();
        Vider();
        ErreurConnexion = "Session expirée : reconnectez-vous.";
        Phase = Phase.Connexion;
    }

    private void Vider()
    {
        Travaux = Array.Empty<Travail>();
        Sorties = Array.Empty<Sortie>();
        Machines = Array.Empty<Machine>();
        Reglages = Array.Empty<Reglage>();
        Systeme = null;
        Selection = null;
        Page = 1;
        ServeurHorsLigne = null;
        _suivi.Reinitialiser();
        _cycle = 0;
    }

    private async Task ChargerReference()
    {
        var api = Api;
        if (api is null) return;
        try { Machines = await api.Machines(); } catch (Exception e) when (e is ErreurApi) { }
        try { Reglages = await api.Reglages(); } catch (Exception e) when (e is ErreurApi) { }
    }

    /// <summary>
    /// Relit les réglages du serveur (à l'ouverture de la feuille d'envoi) : un réglage créé depuis
    /// l'administration après la connexion doit être proposé sans se reconnecter. En cas d'échec,
    /// on garde la liste connue.
    /// </summary>
    public async Task ActualiserReglages()
    {
        var api = Api;
        if (api is null || Phase != Phase.Connecte) return;
        try
        {
            var reglages = await api.Reglages();
            if (Api != api || Phase != Phase.Connecte) return;
            Reglages = reglages;
        }
        catch (Exception e) when (e is ErreurApi) { }
    }

    // ------------------------------------------------------------ Actualisation

    private void LancerBoucle()
    {
        ArreterBoucle();
        var cts = new CancellationTokenSource();
        _boucle = cts;
        _ = Boucle(cts.Token);
    }

    private void ArreterBoucle()
    {
        _boucle?.Cancel();
        _boucle?.Dispose();
        _boucle = null;
    }

    private async Task Boucle(CancellationToken jeton)
    {
        try
        {
            while (!jeton.IsCancellationRequested)
            {
                await Task.Delay(_intervalle, jeton);
                await Rafraichir();
            }
        }
        catch (OperationCanceledException)
        {
            // Arrêt normal : déconnexion, changement de serveur ou fermeture.
        }
    }

    /// <summary>
    /// Une actualisation : travaux et file à chaque tour, les machines de temps en temps (ou à
    /// l'écran Machines), le système seulement à l'écran Système.
    /// </summary>
    public async Task Rafraichir()
    {
        var api = Api;
        if (api is null || Phase != Phase.Connecte) return;
        await _verrou.WaitAsync();
        try
        {
            if (Api != api || Phase != Phase.Connecte) return;
            _cycle++;
            var tTravaux = api.Travaux();
            var tFile = api.File();
            await Task.WhenAll(tTravaux, tFile);
            List<Machine>? machines = null;
            if (Rubrique == Rubrique.Machines || Machines.Count == 0 || _cycle % 10 == 0)
                machines = await api.Machines();
            InfoSysteme? systeme = null;
            if (Rubrique == Rubrique.Systeme)
                systeme = await api.Systeme();
            // Déconnecté ou changé de serveur pendant l'attente : on jette la réponse.
            if (Api != api || Phase != Phase.Connecte) return;
            if (machines is not null) Machines = machines;
            if (systeme is not null) Systeme = systeme;
            Appliquer(tTravaux.Result, tFile.Result);
            ServeurHorsLigne = null;
            Signaler();
        }
        catch (Exception e) when (e is not OperationCanceledException)
        {
            if (Api == api) Traiter(e, silencieux: true);
        }
        finally
        {
            _verrou.Release();
        }
    }

    private void Appliquer(List<Travail> travaux, List<Sortie> sorties)
    {
        foreach (var n in _suivi.Analyser(travaux, sorties, NomMachine))
            NotificationAEnvoyer?.Invoke(n);
        Travaux = travaux;
        Sorties = sorties;
        if (Selection is { } s && !travaux.Any(t => t.Id == s)) Selection = null;
    }

    // ------------------------------------------------------------ Erreurs et messages

    /// <summary>
    /// Message de l'erreur ; certaines exceptions de Windows (COM) n'en ont pas : on donne alors
    /// leur type et leur code, plutôt qu'une boîte « Action impossible » vide.
    /// </summary>
    private static string Texte(Exception e) =>
        string.IsNullOrWhiteSpace(e.Message)
            ? $"Erreur inattendue de Windows ({e.GetType().Name}, code 0x{e.HResult:X8})."
            : e.Message;

    /// <summary>
    /// 401 : retour à l'écran de connexion. Sinon l'erreur est dite à l'opérateur, ou, pendant
    /// l'actualisation (<paramref name="silencieux"/>), seulement notée dans la barre d'état.
    /// </summary>
    public void Traiter(Exception erreur, bool silencieux = false)
    {
        if (erreur is OperationCanceledException) return;
        if (erreur is ErreurApi { ConnexionRequise: true })
        {
            RetourConnexion();
        }
        else if (silencieux)
        {
            ServeurHorsLigne = Texte(erreur);
        }
        else
        {
            Message = new MessageUtilisateur("Action impossible", Texte(erreur));
        }
        Signaler();
    }

    public void FermerMessage()
    {
        if (Message is null) return;
        Message = null;
        Signaler();
    }

    /// <summary>Phrase brève dans la barre d'état, effacée au bout de quelques secondes.</summary>
    public void Annoncer(string texte)
    {
        Annonce = texte;
        Signaler();
        _annonce?.Cancel();
        _annonce?.Dispose();
        var cts = new CancellationTokenSource();
        _annonce = cts;
        _ = EffacerAnnonce(texte, cts.Token);
    }

    private async Task EffacerAnnonce(string texte, CancellationToken jeton)
    {
        try
        {
            await Task.Delay(TimeSpan.FromSeconds(5), jeton);
            if (Annonce == texte) { Annonce = null; Signaler(); }
        }
        catch (OperationCanceledException)
        {
            // Une annonce plus récente a pris la place.
        }
    }

    // ------------------------------------------------------------ Actions sur les travaux

    /// <summary>Exécute un appel d'API, traite l'erreur, puis actualise. Rend vrai si tout a réussi.</summary>
    private async Task<bool> Tenter(Func<Api, Task> operation)
    {
        var api = Api;
        if (api is null) return false;
        try
        {
            await operation(api);
            await Rafraichir();
            return true;
        }
        catch (Exception e) when (e is not OperationCanceledException)
        {
            Traiter(e);
            return false;
        }
    }

    public async Task Imprimer(Travail t, int page, Machine m)
    {
        var operation = Regles.Operation(t);
        if (await Tenter(api => api.MettreEnFile(t.Id, page, m.Id, operation)))
            Annoncer($"Page {page} de « {t.Nom} » mise en file sur {m.Nom}.");
    }

    public async Task ImprimerToutes(Travail t, Machine m)
    {
        var pages = Regles.PagesAImprimer(t);
        var operation = Regles.Operation(t);
        var ok = await Tenter(async api =>
        {
            foreach (var p in pages) await api.MettreEnFile(t.Id, p, m.Id, operation);
        });
        if (ok) Annoncer($"{pages.Count} page(s) de « {t.Nom} » mise(s) en file sur {m.Nom}.");
    }

    public async Task Relancer(Travail t)
    {
        if (await Tenter(api => api.Relancer(t.Id))) Annoncer($"Rendu de « {t.Nom} » relancé.");
    }

    public async Task Supprimer(Travail t)
    {
        if (await Tenter(api => api.Supprimer(t.Id)))
        {
            if (Selection == t.Id) Selectionner(null);
            Annoncer($"« {t.Nom} » supprimé.");
        }
    }

    /// <summary>
    /// Bouton Suspendre de la barre de commandes : suspend les sorties en attente du travail,
    /// ou, s'il n'en reste pas, reprend les suspendues.
    /// </summary>
    public async Task BasculerSuspension(Travail t)
    {
        var suspendre = PeutSuspendre(t);
        var cible = suspendre ? EtatSortie.EnAttente : EtatSortie.Suspendue;
        var action = suspendre ? ActionSortie.Suspendre : ActionSortie.Reprendre;
        var ids = Sorties.Where(s => s.Travail == t.Id && s.Etat == cible).Select(s => s.Id).ToList();
        if (ids.Count == 0) return;
        await Tenter(async api =>
        {
            foreach (var id in ids) await api.Agir(id, action);
        });
    }

    public Task Agir(Sortie s, ActionSortie action) => Tenter(api => api.Agir(s.Id, action));

    /// <summary>
    /// Télécharge l'épreuve TIFF d'une page dans un fichier temporaire et en rend le chemin ;
    /// l'interface propose ensuite de l'enregistrer (puis supprime le temporaire). Null en cas d'échec
    /// (l'erreur est déjà dite à l'opérateur).
    /// </summary>
    public async Task<string?> PreparerEpreuve(Travail t, int page)
    {
        var api = Api;
        if (api is null) return null;
        Annoncer("Préparation de l'épreuve TIFF…");
        var temporaire = Path.Combine(Path.GetTempPath(), $"eko-epreuve-{Guid.NewGuid():N}.tif");
        try
        {
            await api.TelechargerEpreuve(t.Id, page, temporaire);
            Annonce = null;
            Signaler();
            return temporaire;
        }
        catch (Exception e) when (e is not OperationCanceledException)
        {
            Annonce = null;
            Traiter(e);
            return null;
        }
    }

    // ------------------------------------------------------------ Envoi

    /// <summary>Fichiers choisis ou déposés : ceux qui sont acceptés ouvrent la feuille d'envoi.</summary>
    public void ProposerEnvoi(IEnumerable<string> chemins)
    {
        var tous = chemins.ToList();
        var retenus = Regles.FichiersRetenus(tous);
        if (retenus.Count == 0)
        {
            if (tous.Count > 0)
            {
                Message = new MessageUtilisateur("Envoi impossible", Libelles.Erreur("format_inconnu", null));
                Signaler();
            }
            return;
        }
        FichiersProposes?.Invoke(retenus);
    }

    /// <summary>Envoie les fichiers l'un après l'autre (jamais deux envois de plusieurs Go en même temps).</summary>
    public void LancerEnvoi(IReadOnlyList<string> fichiers, string? reglage, string? client, string? commande, string? quantite)
    {
        var api = Api;
        if (api is null) return;
        if (!string.IsNullOrEmpty(reglage) && _parametres.DernierReglage != reglage)
        {
            _parametres.DernierReglage = reglage;
            _enregistrer(_parametres);
        }
        var champs = Regles.ChampsEnvoi(reglage, client, commande, quantite);
        var cts = new CancellationTokenSource();
        var nouveaux = fichiers.Select(f => new EnvoiEnCours(Path.GetFileName(f)) { Annulation = cts }).ToList();
        _envois.AddRange(nouveaux);
        Signaler();
        _ = Envoyer(api, fichiers, nouveaux, champs, cts);
    }

    private async Task Envoyer(Api api, IReadOnlyList<string> fichiers, List<EnvoiEnCours> envois, List<(string Nom, string Valeur)> champs, CancellationTokenSource cts)
    {
        for (var i = 0; i < fichiers.Count; i++)
        {
            var envoi = envois[i];
            if (cts.IsCancellationRequested)
            {
                envoi.Erreur = "Annulé";
                Signaler();
                continue;
            }
            try
            {
                // Progress se rattache au contexte courant (le fil de l'interface).
                var progression = new Progress<double>(v => { envoi.Progression = v; Signaler(); });
                var travail = await api.Envoyer(fichiers[i], champs, progression, cts.Token);
                envoi.Progression = 1;
                envoi.Termine = true;
                Signaler();
                _ = ApresEnvoi(api, travail, envoi);
            }
            catch (OperationCanceledException)
            {
                envoi.Erreur = "Annulé";
                Signaler();
            }
            catch (Exception e)
            {
                envoi.Erreur = Texte(e);
                if (e is ErreurApi { ConnexionRequise: true } && Api == api) Traiter(e);
                Signaler();
            }
        }
        cts.Dispose();
    }

    private async Task ApresEnvoi(Api api, Travail t, EnvoiEnCours envoi)
    {
        if (Api != api) return;
        _suivi.Suivre(t.Id);
        Rubrique = Rubrique.Travaux;
        Selection = t.Id;
        Page = 1;
        Signaler();
        await Rafraichir();
        await Task.Delay(TimeSpan.FromSeconds(2));
        _envois.Remove(envoi);
        Signaler();
    }

    public void AnnulerEnvois()
    {
        foreach (var e in _envois.Where(e => e.EnCours)) e.Annulation?.Cancel();
    }

    public void Fermer(EnvoiEnCours envoi)
    {
        _envois.Remove(envoi);
        Signaler();
    }

    // ------------------------------------------------------------ Fin

    /// <summary>Arrête l'actualisation et libère la connexion (fermeture de l'application, tests).</summary>
    public void Dispose()
    {
        ArreterBoucle();
        _annonce?.Cancel();
        AnnulerEnvois();
        Api?.Dispose();
    }
}
