using System.Runtime.InteropServices;

namespace EkoRip.Client;

/// <summary>
/// Boîte « Ouvrir » de Windows (IFileOpenDialog) en sélection multiple. Remplace
/// <c>FileOpenPicker.PickMultipleFilesAsync</c>, qui échoue (COMException 0x80004005, sans
/// message) dans une application WinUI 3 non empaquetée dès qu'on choisit plusieurs fichiers.
/// </summary>
internal static class SelecteurFichiers
{
    private const uint FosForceFileSystem = 0x40;
    private const uint FosAllowMultiSelect = 0x200;
    private const uint FosPathMustExist = 0x800;
    private const uint FosFileMustExist = 0x1000;
    private const uint SigdnFileSysPath = 0x80058000;
    private const int Annule = unchecked((int)0x800704C7); // HRESULT_FROM_WIN32(ERROR_CANCELLED)

    /// <summary>Chemins choisis ; liste vide si l'opérateur annule.</summary>
    /// <param name="filtres">Paires (libellé, motifs séparés par « ; »), par exemple ("PDF", "*.pdf").</param>
    public static IReadOnlyList<string> Ouvrir(nint fenetre, string titre, IEnumerable<(string Libelle, string Motifs)> filtres)
    {
        var dialogue = (IFileOpenDialog)new FileOpenDialog();
        try
        {
            var specs = filtres.Select(f => new FiltreCom { Nom = f.Libelle, Motif = f.Motifs }).ToArray();
            if (specs.Length > 0) dialogue.SetFileTypes((uint)specs.Length, specs);
            dialogue.GetOptions(out var options);
            dialogue.SetOptions(options | FosForceFileSystem | FosAllowMultiSelect | FosPathMustExist | FosFileMustExist);
            dialogue.SetTitle(titre);

            var hr = dialogue.Show(fenetre);
            if (hr == Annule) return Array.Empty<string>();
            Marshal.ThrowExceptionForHR(hr);

            dialogue.GetResults(out var resultats);
            resultats.GetCount(out var nombre);
            var chemins = new List<string>((int)nombre);
            for (uint i = 0; i < nombre; i++)
            {
                resultats.GetItemAt(i, out var element);
                element.GetDisplayName(SigdnFileSysPath, out var chemin);
                if (!string.IsNullOrEmpty(chemin)) chemins.Add(chemin);
                Marshal.ReleaseComObject(element);
            }
            Marshal.ReleaseComObject(resultats);
            return chemins;
        }
        finally
        {
            Marshal.ReleaseComObject(dialogue);
        }
    }

    [ComImport, Guid("DC1C5A9C-E88A-4dde-A5A1-60F82A20AEF7")]
    private class FileOpenDialog { }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct FiltreCom
    {
        [MarshalAs(UnmanagedType.LPWStr)] public string Nom;
        [MarshalAs(UnmanagedType.LPWStr)] public string Motif;
    }

    // Ordre des méthodes = ordre de la table virtuelle (IModalWindow, IFileDialog, IFileOpenDialog).
    [ComImport, Guid("d57c7288-d4ad-4768-be02-9d969532d960"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IFileOpenDialog
    {
        [PreserveSig] int Show(nint parent);
        void SetFileTypes(uint nombre, [MarshalAs(UnmanagedType.LPArray)] FiltreCom[] filtres);
        void SetFileTypeIndex(uint index);
        void GetFileTypeIndex(out uint index);
        void Advise(nint evenements, out uint cookie);
        void Unadvise(uint cookie);
        void SetOptions(uint options);
        void GetOptions(out uint options);
        void SetDefaultFolder(IShellItem dossier);
        void SetFolder(IShellItem dossier);
        void GetFolder(out IShellItem dossier);
        void GetCurrentSelection(out IShellItem element);
        void SetFileName([MarshalAs(UnmanagedType.LPWStr)] string nom);
        void GetFileName([MarshalAs(UnmanagedType.LPWStr)] out string nom);
        void SetTitle([MarshalAs(UnmanagedType.LPWStr)] string titre);
        void SetOkButtonLabel([MarshalAs(UnmanagedType.LPWStr)] string libelle);
        void SetFileNameLabel([MarshalAs(UnmanagedType.LPWStr)] string libelle);
        void GetResult(out IShellItem element);
        void AddPlace(IShellItem element, int place);
        void SetDefaultExtension([MarshalAs(UnmanagedType.LPWStr)] string extension);
        void Close(int hr);
        void SetClientGuid(ref Guid guid);
        void ClearClientData();
        void SetFilter(nint filtre);
        void GetResults(out IShellItemArray elements);
        void GetSelectedItems(out IShellItemArray elements);
    }

    [ComImport, Guid("43826d1e-e718-42ee-bc55-a1e261c37bfe"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IShellItem
    {
        void BindToHandler(nint contexte, ref Guid gestionnaire, ref Guid interfaceVoulue, out nint resultat);
        void GetParent(out IShellItem parent);
        void GetDisplayName(uint forme, [MarshalAs(UnmanagedType.LPWStr)] out string nom);
        void GetAttributes(uint masque, out uint attributs);
        void Compare(IShellItem autre, uint indication, out int ordre);
    }

    [ComImport, Guid("b63ea76d-1f85-456f-a19c-48159efa858b"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IShellItemArray
    {
        void BindToHandler(nint contexte, ref Guid gestionnaire, ref Guid interfaceVoulue, out nint resultat);
        void GetPropertyStore(int options, ref Guid interfaceVoulue, out nint magasin);
        void GetPropertyDescriptionList(nint cle, ref Guid interfaceVoulue, out nint liste);
        void GetAttributes(int mode, uint masque, out uint attributs);
        void GetCount(out uint nombre);
        void GetItemAt(uint index, out IShellItem element);
        void EnumItems(out nint enumerateur);
    }
}
