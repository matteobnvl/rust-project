# Documentation: src/main.rs

But du module
- Point d’entrée de l’application (fonction `main`).
- Instancie la carte, les robots, la base et la boucle de rendu TUI avec Ratatui.
- Gère l’état global du jeu via la structure `GameState` et la mise à jour à chaque tick.

Types et alias
- SimulationError: enum d’erreurs locales (actuellement embranche uniquement Io(io::Error)). Sert à unifier les erreurs des appels système (crossterm, terminal…).
- Result<T> = std::result::Result<T, SimulationError>: alias pratique pour les fonctions du module.

Structure GameState
Champs
- map: Vec<Vec<map::Tile>>
  Carte courante. Chaque case est un `Tile` (voir map.rs). Mise à jour par les robots (découvertes, ressources consommées, base centrée).
- width: u16, height: u16
  Dimensions « visibles » de la carte (correspondent à la taille du terminal).
- robots: Vec<robot::Robot>
  Liste des robots de la simulation (éclaireurs et collecteurs).
- map_discovered: HashMap<(u16, u16), map::Tile>
  Connaissances globales agrégées des éclaireurs sur les cases découvertes et ressources trouvées.
- _base: base::SharedBase
  Référence partagée (Arc) vers la base. Le champ est préfixé d’un underscore car il n’est pas encore utilisé dans `GameState::update` mais le système de base tourne en tâche asynchrone.

Méthodes
- fn new(map, width, height, robots, base) -> Self
  Construit l’état du jeu initial avec la carte, dimensions, robots et référence vers la base. Initialise `map_discovered` vide.
- fn update(&mut self)
  Boucle de mise à jour par tick:
  1) Déplace chaque robot éclaireur avec `robot::move_robot` puis agrège sa carte découverte dans `map_discovered`.
  2) Calcule l’ensemble des positions de ressources « réservées » par les collecteurs (évite que plusieurs visent la même cible).
  3) Pour chaque robot, transmet la carte globale découverte via `robot::get_discovered_map`.
  4) Pour les collecteurs:
     - Si pas de cible (`target_resource`), cherche la ressource accessible la plus proche avec `robot::find_nearest_resource` en excluant les positions déjà réservées.
     - Si une cible est définie, lance/continue la collecte via `robot::collect_resources`.
  5) Recrée en permanence un bloc 3×3 de `Tile::Base` au centre (width/2, height/2) pour visualiser la base.
  Remarques: Le marquage direct des robots sur la map est commenté (conservé à titre d’exemple).

Fonctions libres
- async fn main() -> Result<()>
  - Configure le logger (via utils), crée les canaux mpsc/broadcast pour la base et lance la tâche `base.run` en arrière-plan.
  - Initialise le terminal Ratatui et récupère ses dimensions comme surface de simulation.
  - Génère une carte de bruits (`map::generate_map`) et place des ressources aléatoires (`map::generate_sources_rand`).
  - Place 3×3 cases `Tile::Base` au centre.
  - Crée 2 robots éclaireurs et 2 collecteurs, construit `GameState` et démarre la boucle `run`.
  - Restaure le terminal à la sortie.
- fn run(terminal, game_state, area) -> Result<()>
  Boucle principale:
  - Définition d’un TICK_RATE = 50 ms. À chaque tick: `game_state.update()`.
  - Écoute des événements clavier (appui sur espace pour quitter).
  - Dessine la carte à chaque frame via `render_map_simple`.
- fn render_map_simple(f, game_state, area)
  - Transforme la matrice `map` en lignes de `Span` avec des caractères/couleurs par type de `Tile`.
  - Affiche aussi les robots en surimpression: X rouge (éclaireur), Y blanc (collecteur).

Interactions clés
- Dépend de map.rs (type Tile et génération), robot.rs (mouvements/collecte), base.rs (canaux et état de base), utils.rs (logger / terminal Ratatui, non documenté ici).