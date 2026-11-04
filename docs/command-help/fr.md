## Optimisation PNG par lot

`pngopt [-all] <file-or-directory> [...]`

Traite uniquement les fichiers directement dans le dossier et déduplique les chemins. Supprime iTXt sur place. Le canal alpha est retiré uniquement des PNG statiques RGBA 32 bits dont tous les pixels ont une valeur alpha de 255.

| Options | |
| --- | --- |
| `-all` | Inclure les sous-dossiers en ignorant les liens symboliques et les jonctions |
| `-h, --help` | Afficher l’aide |
| `--` | Traiter les arguments restants comme des chemins |

### Exemples PowerShell

```powershell
.\unity_yaml_viewer.exe pngopt "a.png" "b.png" "Assets\UI"
```

```powershell
.\unity_yaml_viewer.exe pngopt -all "Assets\UI"
```

État du résultat: `alpha`, `itxt`, `both`, `unchanged`, `failed`.

## Compactage 9-slice par lot

`slice9 <file-or-directory> [...] [options]`

Prend en charge PNG, TGA, BMP et WebP statique. Les résultats plus petits remplacent les originaux sans sauvegarde. Les fichiers .meta compatibles pour un Sprite unique peuvent être actualisés. Utilisez `--dry-run` pour vérifier le plan.

| Options | |
| --- | --- |
| `-r, --recursive` | Inclure les sous-dossiers |
| `--preview` | Ouvrir jusqu’à 10 aperçus modifiables sans écriture automatique |
| `--dry-run` | Calculer le plan sans écrire de fichier |
| `--center WxH` | Centre conservé pour l’étirement, 4x4 par défaut |
| `--repeat-count XxY` | Cycles conservés pour la répétition, 1x1 par défaut |
| `--direction auto / both / horizontal / vertical` | Choisir la direction du compactage |
| `--mode auto / stretch / repeat` | Choisir le mode de compactage |
| `--no-preserve-edges` | Autoriser le rognage des marges transparentes |
| `--no-align4` | Désactiver l’alignement sur les multiples de quatre |
| `--report FILE` | Enregistrer un rapport JSON dans un nouveau fichier |
| `-h, --help` | Afficher l’aide |
| `--` | Traiter les arguments restants comme des chemins |

### Exemples PowerShell

```powershell
.\unity_yaml_viewer.exe slice9 "Assets\UI" -r --dry-run
```

```powershell
.\unity_yaml_viewer.exe slice9 "a.png" "b.png" --preview
```

## Ouvrir des fichiers

Entourez de guillemets chaque chemin contenant des espaces ou des caractères spéciaux.

```powershell
.\unity_yaml_viewer.exe "a.png" "Project Assets\b.prefab"
```

## Résultats et codes de sortie

Les deux commandes affichent du JSON avec les résultats par fichier. Le code 0 indique l’absence d’échec, 1 un échec de fichier et 2 une erreur d’argument ou de rapport. Dans PowerShell, `Start-Process -Wait -PassThru` permet d’attendre et de lire le code de sortie.
