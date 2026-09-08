# Sécurité du fork Windows

La revue porte sur le code source Windows et les modifications du fork. Ce n’est pas une certification ni une garantie d’absence de vulnérabilité.

## Mesures appliquées

- Échappement des données affichées, filtrage des icônes SVG et politique CSP locale sans script distant ni accès Internet depuis les pages du panneau.
- Requêtes aux services attendus, sans suivi des redirections HTTP.
- Clé DeepSeek dans une entrée dédiée du coffre Windows ; saisie masquée et aucune commande IPC de lecture de la clé.
- Modifications des connexions réservées à la fenêtre locale `settings`, avec liste fermée d’identifiants de services.
- Désactivation persistante : anciens relevés masqués, prochaines lectures arrêtées. Une requête déjà engagée peut se terminer.
- Serveur de hooks et collecteurs d’activité non démarrés ; diagnostics détaillés désactivés.
- Actions CI référencées par révision complète. Aucun profil utilisateur ni identifiant personnel dans le paquet partagé.

## Destinations réseau

| Lecteur | Destination |
|---|---|
| Claude | `api.anthropic.com/api/oauth/usage` |
| Codex | `chatgpt.com/backend-api/wham/usage` |
| Cursor | `cursor.com/api/usage-summary` |
| Antigravity | Pont local `127.0.0.1`, puis `cloudcode-pa.googleapis.com` |
| DeepSeek | `api.deepseek.com/user/balance` |

Les boutons de compte ouvrent une adresse officielle fixe dans le navigateur. Certaines interfaces de quotas peuvent changer sans préavis.

## Limites

Les secrets existent temporairement en mémoire pendant leur utilisation. Le coffre ne protège pas d’un logiciel malveillant exécuté sous le même compte Windows. Les journaux et relevés locaux peuvent révéler l’utilisation et les chemins : relisez-les avant de les partager.

La revue du verrou Cargo a signalé `glib 0.18.5`, attendu uniquement pour d’autres plateformes. Le workflow échoue s’il apparaît dans l’arbre Windows. Des dépendances transitives anciennes ou non maintenues subsistent, notamment `proc-macro-error` et des bibliothèques `unic`. Ce contrôle ne remplace pas un audit complet des dépendances ou du binaire.

Les tests vérifient les formats de solde DeepSeek, le rejet d’injection dans une clé et les identifiants de fournisseurs. Aucun constat de vol d’identifiants dans le code examiné ; cela ne garantit pas qu’aucun risque ne subsiste.
