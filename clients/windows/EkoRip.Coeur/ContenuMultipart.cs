using System.Net;
using System.Text;

namespace EkoRip.Coeur;

/// <summary>
/// Corps <c>multipart/form-data</c> envoyé en flux : les champs de fiche d'abord, le fichier en
/// dernier (comme l'attend le serveur), lu par blocs depuis le disque. Le fichier n'est jamais
/// entier en mémoire, et la longueur est connue d'avance (pas de transfert « chunked »).
/// </summary>
public sealed class ContenuMultipart : HttpContent
{
    private const int TailleBloc = 256 * 1024;

    private readonly byte[] _entete;
    private readonly byte[] _fin;
    private readonly Stream _fichier;
    private readonly long _longueurFichier;
    private readonly IProgress<double>? _progression;
    private double _derniere = -1;

    /// <param name="champs">Champs texte, dans l'ordre d'envoi.</param>
    /// <param name="nomFichier">Nom du fichier, tel que le serveur le verra.</param>
    /// <param name="fichier">Flux du fichier ; il est fermé avec le contenu.</param>
    /// <param name="longueurFichier">Taille du fichier en octets.</param>
    /// <param name="frontiere">Frontière du multipart.</param>
    /// <param name="progression">Reçoit une fraction de 0 à 1.</param>
    public ContenuMultipart(
        IEnumerable<(string Nom, string Valeur)> champs,
        string nomFichier,
        Stream fichier,
        long longueurFichier,
        string frontiere,
        IProgress<double>? progression = null)
    {
        _fichier = fichier;
        _longueurFichier = longueurFichier;
        _progression = progression;

        var sb = new StringBuilder();
        foreach (var (nom, valeur) in champs)
        {
            sb.Append("--").Append(frontiere).Append("\r\n");
            sb.Append("Content-Disposition: form-data; name=\"").Append(nom).Append("\"\r\n\r\n");
            sb.Append(valeur).Append("\r\n");
        }
        var nomPropre = nomFichier.Replace("\"", "%22").Replace("\r", "").Replace("\n", "");
        sb.Append("--").Append(frontiere).Append("\r\n");
        sb.Append("Content-Disposition: form-data; name=\"fichier\"; filename=\"").Append(nomPropre).Append("\"\r\n");
        sb.Append("Content-Type: application/octet-stream\r\n\r\n");
        _entete = Encoding.UTF8.GetBytes(sb.ToString());
        _fin = Encoding.ASCII.GetBytes($"\r\n--{frontiere}--\r\n");

        Headers.TryAddWithoutValidation("Content-Type", $"multipart/form-data; boundary={frontiere}");
    }

    private long Total => _entete.Length + _longueurFichier + _fin.Length;

    protected override bool TryComputeLength(out long length)
    {
        length = Total;
        return true;
    }

    protected override Task SerializeToStreamAsync(Stream stream, TransportContext? context) =>
        SerializeToStreamAsync(stream, context, CancellationToken.None);

    protected override async Task SerializeToStreamAsync(Stream stream, TransportContext? context, CancellationToken cancellationToken)
    {
        // Une nouvelle tentative (redirection, authentification) relit le fichier depuis le début.
        if (_fichier.CanSeek) _fichier.Position = 0;

        long envoye = 0;
        await stream.WriteAsync(_entete, cancellationToken).ConfigureAwait(false);
        envoye += _entete.Length;

        var bloc = new byte[TailleBloc];
        long restant = _longueurFichier;
        while (restant > 0)
        {
            var lu = await _fichier.ReadAsync(bloc.AsMemory(0, (int)Math.Min(bloc.Length, restant)), cancellationToken).ConfigureAwait(false);
            if (lu == 0) throw new IOException("Le fichier s'est raccourci pendant l'envoi.");
            await stream.WriteAsync(bloc.AsMemory(0, lu), cancellationToken).ConfigureAwait(false);
            restant -= lu;
            envoye += lu;
            Signaler(envoye);
        }

        await stream.WriteAsync(_fin, cancellationToken).ConfigureAwait(false);
        Signaler(Total);
    }

    /// <summary>Au plus ~200 notifications par envoi.</summary>
    private void Signaler(long envoye)
    {
        if (_progression is null || Total <= 0) return;
        var fraction = (double)envoye / Total;
        if (fraction - _derniere >= 0.005 || fraction >= 1)
        {
            _derniere = fraction;
            _progression.Report(fraction);
        }
    }

    protected override void Dispose(bool disposing)
    {
        if (disposing) _fichier.Dispose();
        base.Dispose(disposing);
    }
}
