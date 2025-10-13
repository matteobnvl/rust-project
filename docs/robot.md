# Documentation: src/robot.rs

But du module
- Définir les robots (éclaireurs/collecteurs), leur position, leur « vision », l’exploration, le pathfinding et la logique de collecte.

Structures et enums
- struct Robot
  Champs:
  - position: RobotPosition (x, y)
  - energy: u32 (non consommée actuellement, prévue pour extensions)
  - robot_type: RobotType (Eclaireur | Collecteur)
  - map_discovered: HashMap<(u16,u16), Tile> connaissances locales du robot (copiées depuis l’agrégat global pour les collecteurs, enrichies pour les éclaireurs)
  - found_resources: bool indique qu’une ressource a été repérée et qu’il faut rentrer à la base (éclaireur) ou qu’on transporte (collecteur)
  - collected_resources: u32 compteur de ressources collectées par le robot
  - target_resource: Option<RobotPosition> position de la ressource visée (collecteur)
- enum RobotType { Eclaireur, Collecteur }
- struct RobotPosition(pub u16, pub u16)
  Tuple struct utilisable en HashMap/HashSet; implémente Eq/Hash/Copy/Clone/Debug.
  Méthodes:
  - distance(&self, other) -> u16: distance de Manhattan.
  - successors(&self) -> Vec<(RobotPosition, u16)>: voisins 4‑connexité avec coût 1 (borné à x,y >= 0; la validation des limites supérieures se fait ailleurs).

Constructeurs de robots
- fn robots_eclaireur(width, height) -> Robot
  Crée un éclaireur positionné au centre (width/2, height/2), énergie 100.
- fn robots_collecteur(width, height) -> Robot
  Crée un collecteur centré, énergie 100.

Perception et mise à jour de carte
- fn robot_vision(robot, map, width, height) -> HashMap<(u16,u16), Tile>
  Balaye un carré centré sur la position du robot avec une distance de vue croissante jusqu’à 3 lorsque le robot est sur la base (centre). Ajoute chaque tuile vue aux connaissances temporaires.

Collecte et mouvement haut-niveau
- fn collect_resources(robot, target, map, width, height)
  Logique d’un collecteur:
  - Si la case visée est déjà `Explored`, abandonne la cible et rentre à la base.
  - Si `found_resources` et robot est à la base: réinitialise cible/état.
  - Sinon, si `found_resources` et robot n’est pas à la base: se dirige vers la base.
  - Sinon, si la cible n’est pas marquée `Explored` dans `map_discovered`: se dirige vers la cible.
  - Lorsqu’il atteint la cible: décrémente la quantité sur `SourceFound`/`CristalFound`, incrémente `collected_resources`, marque `Explored` si quantité 0 et efface la cible.
- fn get_discovered_map(robot, discovered)
  Copie l’agrégat global découvert dans la carte locale du robot (utile aux collecteurs).
- fn go_to_nearest_point(robot, target)
  Utilise A* depuis `robot.position` vers `target` en autorisant les déplacements via les cases `Explored` (ou la case cible). Avance d’un pas sur le chemin si trouvé.

Exploration avec BFS
- fn move_robot(robot, map, width, height)
  Logique d’un éclaireur par tick:
  1) Marque la case courante comme `Explored` si `Floor` ou `Base` et l’inscrit dans `map_discovered`.
  2) Calcule la vision (`robot_vision`) et pour chaque tuile vue:
     - `Source` -> convertit en `SourceFound` dans la carte globale et locale.
     - `Cristal` -> convertit en `CristalFound`.
     - `Floor`/`Base`/`Explored` -> marque comme `Explored` localement.
  3) Met à jour `found_resources` si des ressources sont vues; s’il est à la base, réinitialise ce flag.
  4) Si `found_resources` et pas à la base: revient vers la base via `go_to_nearest_point`.
  5) Sinon, cherche une case non explorée accessible via BFS à partir de la position actuelle en se limitant aux cases `Floor|Explored|Base`. Avance d’un pas sur le chemin s’il existe; sinon log d’info.

Recherche de ressource
- fn find_nearest_resource(robot, discovered, reserved) -> Option<RobotPosition>
  Parcourt `discovered` pour trouver la ressource la plus proche (CristalFound/SourceFound) non réservée par un autre collecteur, minimise la distance de Manhattan depuis le robot. Retourne None s’il n’y en a pas.