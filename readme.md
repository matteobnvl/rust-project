# Rust Resource Simulation

Une simulation interactive dans le terminal, écrite en **Rust**.  
Des robots explorent une carte générée procéduralement, découvrent des ressources et les rapportent à une base centrale.  
L’interface est rendue dans le terminal grâce à **ratatui**.

- Les **éclaireurs** (Scouts) explorent la carte en utilisant une expansion de frontière guidée par BFS et un pathfinding A*, tout en partageant leurs découvertes pour éviter les doublons.
- Les **collecteurs** (Collectors) réservent les ressources découvertes afin qu’un seul collecteur se rende à une ressource donnée à la fois.
- La **Base** agrège les découvertes et les ressources collectées, tout en diffusant les nouvelles positions de ressources aux collecteurs.

Appuyez sur **n’importe quelle touche** pour quitter la simulation.

---

## Démarrage rapide

### Prérequis
- Rust (stable) et Cargo installés → [https://rustup.rs](https://rustup.rs)
- Un terminal compatible avec le rendu **ANSI** (couleurs et symboles).

Build:
```bash
cargo build
```

Run:
```bash
cargo run
```

Run in release mode (faster):
```bash
cargo run --release
```

---
## Architecture du projet

Le code source se trouve dans le dossier `src/` :

- `main.rs` — Initialise la journalisation et l’interface terminal, construit la carte et l’état partagé, lance le runtime de simulation et exécute la boucle de rendu.
- `map.rs` — Représente la carte sous forme de grille, génère les obstacles avec du bruit de Perlin, place la base, les ressources, et fournit les fonctions d’aide au pathfinding A*.
- `base.rs` — Logique centrale de la base : gestion des messages, file des ressources connues, système de réservation et statistiques globales.
- `robots.rs` — Gère les deux types de robots (Éclaireurs et Collecteurs), leurs boucles asynchrones, l’état partagé, la mémoire d’exploration (visited/frontier) et le mécanisme de revendication de cibles.
- `simulation.rs` — Lance les tâches asynchrones (base + robots) dans un runtime Tokio multithread et connecte les états partagés.
- `ui.rs` — Rendu de l’interface terminal avec **ratatui** : dessine la grille, affiche les positions des robots et les totaux dans une barre d’état.
- `utils.rs` — Configuration du système de logs.

### Documentation supplémentaire
Une documentation détaillée est disponible dans le dossier `docs/`, avec un fichier par module :
- `docs/main.md`, `docs/map.md`, `docs/base.md`, `docs/robots.md`, `docs/simulation.md`, `docs/ui.md`.

---

## Fonctionnement général

### Vue d’ensemble

1. **Génération de la carte**  
   La carte est adaptée à la taille du terminal.  
   Des obstacles sont placés à l’aide de bruit de Perlin, une base 3x3 est centrée, et les ressources (`Energy`, `Crystal`) sont dispersées sur les cases vides.

2. **Lancement de la simulation**  
   La simulation démarre une tâche pour la base, deux pour les éclaireurs et deux pour les collecteurs, via le runtime Tokio.  
   Tous les robots commencent à la base.

3. **Exploration (Scouts)**
   - Partagent un ensemble de cases visitées et une frontière BFS commune pour coordonner la couverture de la carte.
   - Lors du choix d’une nouvelle cible, utilisent une fonction de score et un mécanisme de revendication (`claim`) pour éviter de converger vers la même case.
   - Se déplacent à l’aide du pathfinding **A*** sur une grille à quatre directions.
   - Lorsqu’un éclaireur découvre une ressource, il la met en attente, retourne à la base pour la rapporter, et c’est seulement à ce moment que la découverte est diffusée et enregistrée.

4. **Collecte (Collectors)**
   - La base maintient une file de ressources connues et une liste de réservations.
   - Un collecteur réserve une ressource de façon atomique avant de s’y rendre, garantissant l’exclusivité.
   - À l’arrivée, il tente de collecter une unité par tick ; si la case est vide, la base supprime la ressource pour éviter toute réassignation.
   - Les unités collectées sont rapportées à la base, qui met à jour ses totaux.

5. **Interface utilisateur (UI)**  
   L’interface redessine continuellement la carte et les positions des robots.  
   Appuyez sur **n’importe quelle touche** pour quitter la simulation.

---

## Algorithmes et structures clés

- **A\*** (plus court chemin) avec heuristique de Manhattan (coût uniforme = 1 par déplacement).
- **Frontière BFS partagée** (`VecDeque`) et ensemble `visited` (`HashSet`) protégés par des `RwLock` asynchrones.
- **Mécanisme de revendication** des cibles pour répartir les éclaireurs.
- **Liste de réservations** côté base pour éviter que deux collecteurs visent la même ressource.
- **Canaux Tokio** (`broadcast` et `mpsc`) pour la communication entre les robots et la base.

---

## Contrôles et conseils

- La simulation continue jusqu’à ce que vous appuyiez sur une touche.
- Redimensionnez votre terminal **avant** le lancement pour obtenir une carte plus grande.  
  (La première ligne du terminal est utilisée pour la barre d’état.)
- Le **mode release** améliore grandement les performances sur les grands terminaux :
  ```bash
  cargo run --release
