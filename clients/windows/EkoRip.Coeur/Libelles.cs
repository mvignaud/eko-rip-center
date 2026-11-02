namespace EkoRip.Coeur;

/// <summary>
/// Libellés français. La table des erreurs est celle de l'interface web
/// (<c>serveur/static/index.html</c>, <c>textes.fr.erreurs</c>), clés de même nom ; elle
/// reprend <c>Libelles.swift</c> du client Mac.
/// </summary>
public static class Libelles
{
    public static readonly IReadOnlyDictionary<string, string> Erreurs = new Dictionary<string, string>
    {
        ["format_inconnu"] = "Format non reconnu : envoyer un PDF, un AI, un SVG ou une image TIFF, PNG ou JPEG.",
        ["image_illisible"] = "Image illisible",
        ["image_non_prise_en_charge"] = "Image non prise en charge",
        ["ai_sans_pdf"] = "Fichier Illustrator sans compatibilité PDF : l'enregistrer avec « Créer un fichier compatible PDF ».",
        ["svg_decoupe_seulement"] = "Un SVG ne peut qu'être découpé : choisir « Découper seulement ».",
        ["mode_invalide"] = "Mode inconnu.",
        ["fichier_vide"] = "Le fichier est vide.",
        ["aucun_fichier"] = "Aucun fichier reçu.",
        ["dpi_invalide"] = "Résolution hors de 36 à 2880 dpi.",
        ["envoi_illisible"] = "Envoi illisible.",
        ["envoi_interrompu"] = "L'envoi a été interrompu.",
        ["erreur_interne"] = "Erreur du serveur.",
        ["introuvable"] = "Élément introuvable.",
        ["connexion_requise"] = "Connexion requise.",
        ["identifiants_invalides"] = "Utilisateur ou mot de passe incorrect.",
        ["trop_d_echecs"] = "Trop d'essais pour cet utilisateur : réessayer dans 5 minutes.",
        ["admin_requis"] = "Réservé à l'administrateur du RIP.",
        ["sauvegarde_invalide"] = "Ce fichier n'est pas une sauvegarde d'E-KO RIP Center.",
        ["rendu_echoue"] = "Le rendu a échoué.",
        ["decoupe_illisible"] = "Tracés de découpe illisibles dans ce PDF.",
        ["profil_inconnu"] = "Profil ICC introuvable sur le serveur.",
        ["profil_inadapte"] = "Profil ICC inadapté (CMJN attendu en entrée CMJN et en sortie, RVB en entrée RVB).",
        ["profil_invalide"] = "Ce fichier n'est pas un profil ICC (.icc ou .icm).",
        ["planche_invalide"] = "Planche impossible.",
        ["mosaique_invalide"] = "Mosaïque impossible.",
        ["les_incoherents"] = "Lés incohérents : largeurs, recouvrements, ordre ou tuiles à imprimer.",
        ["soudure_invalide"] = "Soudure : il faut au moins 12,7 mm de recouvrement sur chaque tuile.",
        ["reglage_invalide"] = "Nom de réglage invalide (lettres, chiffres, espaces, - _ +).",
        ["rien_a_decouper"] = "Ce travail n'a aucun tracé de découpe.",
        ["rien_a_imprimer"] = "Cette page n'a rien à imprimer.",
        ["epreuve_seule"] = "Ce travail est en épreuve seule : il ne s'imprime pas.",
        ["epreuve_invalide"] = "Épreuve TIFF : taille maximale entre 1 et 3500 Mo.",
        ["epreuve_echouee"] = "L'épreuve TIFF n'a pas pu être faite.",
        ["envoi_impossible"] = "Envoi à l'imprimante impossible.",
        ["imprimante_en_alerte"] = "L'imprimante signale une alerte : impression arrêtée.",
        ["travail_occupe"] = "Ce travail est en cours de rendu, ou une sortie l'attend dans la file.",
        ["fiche_invalide"] = "Fiche invalide : 200 caractères par champ, 2000 pour les notes, quantité 100 000 au plus.",
        ["page_exclue"] = "Cette page est exclue de l'impression : recochez-la d'abord.",
        ["rien_a_imbriquer"] = "Rien à imbriquer sur cette planche.",
        ["imbrication_changee"] = "La file a changé depuis l'aperçu : planche recalculée, réessayez.",
        ["planche_rendue_a_la_file"] = "Planche abandonnée : ses travaux sont revenus en file. La refaire les imprimerait deux fois ; « Imprimer maintenant » sur la planche en préparation en refait une.",
        ["sortie_sur_une_planche"] = "Cette page est sur une planche pas encore imprimée : la refaire l'imprimerait deux fois.",
        ["tarifs_invalides"] = "Tarifs invalides : nombres positifs attendus.",
        ["consommation_invalide"] = "Réglages de consommation invalides : rétention de 1 à 3650 jours, facteurs de 0 à 10, une fois chacun.",
        ["pool_incoherent"] = "Pool incohérent : nom d'identifiant libre, machines de même modèle et même laize.",
        ["support_different"] = "Les machines du pool n'ont pas le même support chargé : charger le même support, ou dupliquer la machine.",
        ["paquet_invalide"] = "Paquet support invalide.",
        ["paquet_trop_recent"] = "Paquet fait par une version plus récente du serveur : mettre ce serveur à jour d'abord.",
        ["support_inconnu"] = "Support ou mode inconnu (ou mode d'un autre support).",
        ["espace_insuffisant"] = "Espace disque insuffisant pour ce travail : libérer de la place ou baisser la résolution.",
        ["pas_pret"] = "Pas encore prêt : le rendu de ce travail est en cours.",
        ["jeton_erp_requis"] = "Jeton de l'ERP requis.",
        ["url_erp_invalide"] = "URL de l'ERP invalide : https://hôte/chemin, sans paramètres (http seulement vers cette machine).",
        ["secret_erp_invalide"] = "Secret invalide : 32 à 256 caractères visibles, sans espace.",
        ["pont_incomplet"] = "Pour activer le pont, il faut l'URL de l'ERP et le secret.",
        ["pont_remplace"] = "Ce RIP a été remplacé par une autre installation.",
        ["pont_anomalie_seq"] = "L'ERP a traité plus d'événements que ce RIP n'en a numéroté.",
        ["pont_rejeu_impossible"] = "L'ERP demande des événements déjà purgés : rejeu impossible.",
        ["erp_injoignable"] = "ERP injoignable (réseau, adresse ou certificat).",
        ["erp_reponse_invalide"] = "Réponse de l'ERP illisible.",
        ["erp_http"] = "Erreur de l'ERP.",
        ["signature_absente"] = "ERP : signature absente.",
        ["signature_invalide"] = "ERP : signature refusée (vérifier le secret collé dans l'ERP).",
        ["signature_expiree"] = "ERP : signature expirée (mettre l'horloge du serveur à l'heure).",
        ["pont_non_configure"] = "ERP : le secret n'est pas enregistré dans l'ERP.",
        ["locataire_inconnu"] = "ERP : atelier inconnu (vérifier l'URL de base).",
        ["corps_invalide"] = "ERP : corps refusé.",
        ["requete_invalide"] = "ERP : requête refusée.",
        ["contrat_non_supporte"] = "ERP : version du contrat non prise en charge.",
        ["evenement_invalide"] = "ERP : événement refusé.",
        ["catalogue_invalide"] = "ERP : catalogue refusé.",
        ["seq_manquant"] = "ERP : événements manquants, rejeu en cours.",
        ["jeton_erp_hors_pont"] = "Le jeton de l'ERP n'ouvre que les routes du pont.",
        ["identifiant_inconnu"] = "Identifiant inconnu : un identifiant ne se choisit pas, il est attribué par le serveur.",
        ["supports_invalides"] = "Supports invalides : nom unique (60 caractères), encres connues sans blanc et argent ensemble, au moins un mode (90 à 2880 dpi, 1 à 64 passes, encre totale de 100 à 400 %).",
        ["pdf_illisible"] = "PDF illisible : impossible d'en séparer les pages. Le réenregistrer depuis l'application d'origine.",
        ["epreuve_impossible"] = "Épreuve impossible : elle se fait en impression, sur une autre machine qui imprime.",
        ["jeton_renvoi_requis"] = "Renvoi refusé : jeton absent ou faux (sur ce serveur : eko-rip-serveur renvoi jeton).",
        ["renvoi_vide"] = "Renvoi sans flux.",
        ["renvoi_sans_connexion"] = "Cette machine sort en dossier : elle ne peut pas recevoir de renvoi.",
        ["renvoi_echoue"] = "La machine n'a pas pris le flux renvoyé.",
        ["polices_manquantes"] = "Polices non incorporées : le réglage demande de refuser le fichier. Incorporer les polices ou vectoriser le texte.",
        ["regles_invalides"] = "Règles invalides : motif non vide sans « / », bornes de format ordonnées (0 à 100 000 mm), réglage existant.",
        ["motif_balises_invalide"] = "Motif à balises invalide : balises connues entre accolades, chacune une fois, séparées par du texte (« __ »), sans « / ».",
        ["demande_invalide"] = "Réglages du travail invalides : imprimante inconnue ou répétition hors de 1 à 100.",
        ["mire_invalide"] = "Mire invalide : 5 à 101 plages, encres en lettres et chiffres.",
        ["mesures_invalides"] = "Mesures invalides : au moins 4 aplats d'encre totale différente, L* de 0 à 100.",
        ["blanc_invalide"] = "Blanc : rétraction de 0 à 5 mm, encre minimale de 1 à 100 %.",
        ["client_requis"] = "Réservé au client E-KO RIP Center.",
        ["machine_invalide"] = "Machine invalide (identifiant : minuscules, chiffres et -, unique ; laize 100 à 5000 mm).",
        ["machine_occupee"] = "Une sortie attend encore cette machine dans la file.",
        ["operation_impossible"] = "Cette machine ne fait pas cette opération.",
        ["media_indisponible"] = "L'Epson ne répond pas ou ne donne pas la largeur du média.",
        ["media_non_configure"] = "Lecture du média non configurée.",
        ["recadrage_invalide"] = "Recadrage impossible : le cadre doit faire au moins 1 × 1 mm et rester dans la page.",
        ["oeillets_invalides"] = "Œillets : de 1 à 200 par bord.",
        ["marques_invalides"] = "Marques impossibles (distance 0 à 100 mm, longueur 1 à 100 mm, épaisseur 0,05 à 5 mm, tiret et vide 0,1 à 50 mm, 20 repères de pli au plus).",
        ["filtre_invalide"] = "Filtre couleur impossible (courbes : 16 points au plus, de 0 à 100 %, entrées croissantes ; luminosité et saturation −100 à 100 ; 20 remplacements au plus, plage 0 à 50 %).",
        ["fond_perdu_invalide"] = "Fond perdu impossible (0 à 100 mm par côté, rogne 0 à 200 mm, couleur 0 à 100 %).",
        ["etiquette_invalide"] = "Étiquette impossible (corps 1,5 à 20 mm, en haut ou en bas).",
        ["code_invalide"] = "QR code impossible (côté 5 à 100 mm, blanc 0 à 50 mm, en haut ou en bas).",
        ["edition_invalide"] = "Édition impossible (échelle 1 à 1000 %, marge 0 à 500 mm, œillets : écart 50 à 5000 mm, retrait 3 à 200 mm, diamètre 2 à 50 mm).",
        ["action_impossible"] = "Action impossible dans cet état.",
        ["sortie_echouee"] = "La sortie vers la machine a échoué.",
        ["interrompu"] = "Interrompu par un arrêt du serveur.",
        ["couches_invalides"] = "Couches de découpe invalides : les mêmes couches qu'au rendu, réglage de 40 caractères au plus.",
        ["mise_coupe_invalide"] = "Mise en coupe invalide : rotation 0, 90, 180 ou 270 ; échelle de 1 à 1000 % ; 1 à 100 copies par sens (1000 en tout) ; écart de 0 à 1000 mm ; décalage de 0 à 5 mm.",
        ["mise_decoupe_seule"] = "Miroir, échelle, copies et départ à l'origine : en découpe seule uniquement (la coupe ne suivrait plus l'impression).",
        ["etendre_invalide"] = "Étendre l'image près des tracés : largeur de 0,1 à 10 mm, seuil de 0 à 10 mm.",
        ["variante_invalide"] = "Variante invalide : elle doit reprendre une découpeuse du catalogue qui n'est pas elle-même une variante.",
        ["fond_reperes_invalide"] = "Fond sous les repères : débord de 0,5 à 13 mm, CMJN de 0 à 100 %.",
        ["detourage_invalide"] = "Détourage invalide : seuil de 1 à 255, aire minimale de 0 à 10 000 mm², lissage de 0 à 5, décalage de 0 à 20 mm.",
        ["detourage_impossible"] = "Détourage : seulement sur une page imprimée d'un PDF rendu (il faut une image à détourer).",
        ["prefixes_invalides"] = "Préfixes de découpe invalides : de 1 à 30, séparés par « ; », 40 caractères au plus chacun.",
        ["csv_vide"] = "Le fichier CSV ne contient aucune teinte.",
        ["intention_invalide"] = "Intention inconnue : de 0 (perceptuelle) à 3 (colorimétrique absolue).",
        ["csv_illisible"] = "Fichier CSV illisible : une ligne par teinte, nom;L;a;b.",
        ["lab_invalide"] = "Valeur Lab hors bornes : L de 0 à 100, a et b de −128 à 127.",
        ["nom_vide"] = "Un ton direct (ou un alias) n'a pas de nom.",
        ["nom_reserve"] = "Ce nom a déjà sa propre encre ou sert à la découpe : il ne peut pas entrer dans la table des tons directs.",
        ["nom_en_double"] = "Ce nom de ton direct (ou d'alias) existe déjà dans la table.",
        ["decoupeuse_invalide"] = "Découpeuse invalide.",
        ["decoupeuse_indisponible"] = "Découpeuse indisponible : un réglage de couche n'existe pas sur cette machine.",
        ["trop_gros"] = "Fichier trop volumineux pour le serveur.",
        ["reseau"] = "Serveur injoignable.",
    };

    /// <summary>Erreurs qui naissent dans le client.</summary>
    private static readonly Dictionary<string, string> LocalesClient = new()
    {
        ["erreur_http"] = "Réponse inattendue du serveur.",
        ["reponse_illisible"] = "Réponse du serveur illisible.",
        ["adresse_invalide"] = "Adresse du serveur invalide : hôte:8640.",
        ["fichier_illisible"] = "Fichier illisible.",
    };

    /// <summary>Message d'une erreur du serveur ; code inconnu : le code et le détail tels quels.</summary>
    public static string Erreur(string? code, string? detail)
    {
        code ??= "erreur_interne";
        var extra = string.IsNullOrEmpty(detail) ? "" : $" ({detail})";
        if (Erreurs.TryGetValue(code, out var texte)) return texte + extra;
        if (LocalesClient.TryGetValue(code, out var local)) return local + extra;
        return code + extra;
    }

    /// <summary>Pourquoi une sortie attend : (titre, explication), comme <c>raisons</c> de l'interface web.</summary>
    public static readonly IReadOnlyDictionary<string, (string Titre, string Aide)> Raisons = new Dictionary<string, (string Titre, string Aide)>
    {
        ["attente_impression"] = ("Impression pas finie", "La découpe part quand l'impression de la page est terminée à 100 %."),
        ["decoupeuse_indisponible"] = ("Découpeuse indisponible", "La découpeuse affectée est absente, hors ligne, mal connectée, ou une couche n'y trouve pas son réglage : rien ne part, pour ne pas gâcher d'encre ni de support."),
        ["attente_operateur"] = ("Retenue", "Suspendue par un opérateur : « Reprendre » pour la relancer."),
        ["machine_hors_ligne"] = ("Machine injoignable", "La machine n'est pas branchée ou absente du catalogue : la sortie partira dès qu'elle répond."),
        ["source_absente"] = ("Fichier manquant", "Le rendu ou le tracé de cette page est introuvable : relancer le travail."),
        ["format_trop_petit"] = ("Trop large", "La page (repères compris) dépasse la laize de la machine : tourner, réduire ou changer de support."),
        ["attente_support"] = ("Autre support", "Le support chargé n'est pas celui du travail."),
        ["attente_imbrication"] = ("En imbrication", "L'impression attend sa planche : elle part avec les autres au déclencheur, ou sur « Imprimer maintenant »."),
        ["page_exclue"] = ("Page exclue", "Cette page est décochée : elle ne part pas tant qu'elle n'est pas recochée."),
    };

    public static (string Titre, string Aide) Raison(string code) =>
        Raisons.TryGetValue(code, out var r) ? r : (code, "");

    /// <summary>Texte d'une alerte de pré-vérification (<c>alertes[].code</c> + <c>detail</c>).</summary>
    public static string TexteAlerte(Alerte a)
    {
        var d = a.Detail ?? "";
        var p = a.Page?.ToString() ?? "?";
        return a.Code switch
        {
            "police_non_incorporee" => $"Police non incorporée : {d} — une autre sera prise au rendu. Vectoriser le texte ou incorporer la police.",
            "image_basse_definition" => $"Image peu définie, page {p} : {d} (moins de 100 ppi à la taille imprimée).",
            "image_sans_resolution" => $"L'image ne donne pas sa résolution : posée à {d}. Vérifier la taille, ou la régler dans Réglages (échelle).",
            "image_canaux_ignores" => $"{d} canal(aux) de sélection du TIFF (masques Photoshop) ignoré(s) : ils ne s'impriment pas.",
            "image_tons_directs" => $"Tons directs du TIFF repris en plaques : {d}. Vérifier la plaque avant d'imprimer.",
            "epreuve_echouee" => $"Épreuve TIFF non faite : {d}",
            "rotation_auto" => $"Tourné de 90° pour tenir dans la laize de l'imprimante ({d}).",
            "decoupe_ton_rvb" => $"Page {p} : le ton de découpe {d} est défini en RVB ; le définir en CMJN (ton direct).",
            "decoupe_avec_fond" => $"Page {p} : le tracé {d} a un fond ; un tracé de découpe est un filet seul, sans remplissage.",
            "decoupe_touche_bord" => $"Page {p} : un tracé {d} touche le bord de la page ; l'en écarter.",
            "decoupe_traces_ouverts" => $"Page {p} : {d} tracé(s) ouvert(s) non coupé(s).",
            "tons_directs_fondus" => $"Tons directs imprimés en CMJN d'après la table des tons directs : {d}.",
            "ton_direct_absent" => $"Page {p} : ton direct absent de la table des tons directs : {d}.",
            "marques_neutralisees" => "Repérage Kona : traits de coupe et croix non imprimés (ils gêneraient la lecture des repères).",
            _ => d.Length == 0 ? a.Code : $"{a.Code} : {d}",
        };
    }

    /// <summary>Modèle de machine, comme la liste <c>modeles</c> de l'interface web.</summary>
    public static string Modele(string? m) => m switch
    {
        "epson_sc_s80600" => "Epson SC-S80600 (impression)",
        "mutoh_kona" => "Mutoh Kona (découpe)",
        "graphtec_fc7000" => "Graphtec FC7000 (découpe)",
        "roland_versacamm" => "Roland VersaCAMM VP (impression et découpe)",
        _ => m ?? "Machine",
    };
}
