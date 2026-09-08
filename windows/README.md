# Codenotch Windows 0.4.0

Un panneau au bord de l’écran pour suivre vos quotas et votre solde API DeepSeek. Le menu **⚙ Connexions** est accessible sur le panneau et depuis l’icône de notification Windows.

## Installation

1. Téléchargez l’archive Windows x64 depuis les [Releases du fork](https://github.com/snooowflake/codenotch/releases).
2. Extrayez les fichiers dans un dossier personnel et lancez `codenotch.exe`.
3. Cliquez sur **⚙**, activez les services souhaités et suivez les indications.

Windows 10/11 x64 et Microsoft Edge WebView2 Runtime sont nécessaires. Le binaire portable n’est pas signé : Windows peut afficher un avertissement d’éditeur inconnu. Comparez `Get-FileHash .\codenotch.exe -Algorithm SHA256` au fichier `SHA256SUMS.txt`.

Pour une mise à jour, quittez Codenotch depuis son icône de notification avant de remplacer les fichiers. Réglages et clé enregistrée sont conservés. Le démarrage avec Windows est optionnel, désactivé par défaut.

## Connexions

| Service | Connexion | Affichage |
|---|---|---|
| GPT / Codex | Session ChatGPT de Codex sur ce PC (`codex login` ou application Codex) | Quotas ; dernier relevé local en secours |
| Claude Code | `/login` dans Claude Code | Quotas de session et de semaine |
| Cursor | Connexion dans l’éditeur Cursor | Utilisation du compte de l’éditeur |
| Antigravity | Application ouverte avec votre compte Google connecté | Quotas disponibles, ou compteur de tours indiqué comme tel |
| DeepSeek API | Clé `sk-…` dans **Connexions → DeepSeek API** | Solde officiel USD/CNY |

Le menu permet d’activer/désactiver chaque service, d’actualiser, d’ouvrir les pages officielles et d’enregistrer/remplacer/supprimer la clé DeepSeek. Les autres lecteurs utilisent les sessions locales : saisissez les codes de connexion dans leurs parcours officiels. Une clé API OpenAI, Anthropic ou Gemini ne remplace pas une session d’abonnement pour ces lecteurs.

Les clés déjà enregistrées par `Setup-DeepSeek.ps1` sont reconnues. Ce script reste disponible en secours. **Activer un lecteur ne connecte pas un compte absent.** Une session expirée ou une API modifiée peut empêcher un relevé ; son état ou son âge sont indiqués.

## Données et compilation

La clé DeepSeek est conservée dans le coffre Windows sous `codenotch:deepseek`, jamais renvoyée au formulaire. Les réglages et relevés résident dans `%APPDATA%\codenotch`. Les collecteurs d’activité et le serveur de hooks sont désactivés. Voir [SECURITY.md](SECURITY.md).

Prérequis de compilation : Rust stable MSVC, Visual C++ Build Tools avec SDK Windows, WebView2.

```powershell
cd windows
cargo test --locked --release -p codenotch
cargo build --locked --release -p codenotch
.\target\release\codenotch.exe
```

Le workflow `Windows portable build` teste et compile avec les dépendances verrouillées. Le paquet contient la révision source, les empreintes et l’arbre des dépendances Windows. Aucune clé personnelle n’est nécessaire à la compilation.

## Crédits

Fork de [vinzdg/codenotch](https://github.com/vinzdg/codenotch), port Windows issu de [Im-Midi/codenotch-windows](https://github.com/Im-Midi/codenotch-windows). Licence MIT : `LICENSE` à la racine. Icônes et marques : `codenotch/glyphs/NOTICE.md`.
