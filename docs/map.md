# map.rs — Génération de la carte, accès à la grille et pathfinding

Ce module définit la grille de la carte (les cellules), fournit des accès concurrents sécurisés, place la base et les ressources, et implémente les utilitaires de recherche de chemin (A*) utilisés par les robots.

---

## Types

### `Cell`
**But :** Représente une tuile individuelle sur la carte.

**Variantes :**
- `Empty` → Tuile vide et traversable (aucune ressource).
- `Obstacle` → Tuile non traversable.
- `Energy(u32)` → Tuile traversable contenant une quantité d’énergie.
- `Crystal(u32)` → Tuile traversable contenant une quantité de cristal.
- `Base` → Tuile traversable correspondant à la position de la base (zone 3x3 au centre).

---

### `Map`
**But :** Grille 2D partagée représentant l’état du monde.

**Champs :**
- `width`, `height` → Dimensions de la grille.
- `grid` → `Arc<RwLock<Vec<Vec<Cell>>>>` : stockage partagé et synchronisé de la grille.
- `base_pos` → `(usize, usize)` : position centrale de la base.

---

## Méthodes (création et opérations de base)

### `Map::from_area(area: ratatui::layout::Size) -> Self`
**But :** Créer une carte adaptée à la taille du terminal, réserver une ligne pour la barre de titre, générer les obstacles et ressources, et placer la base (3x3) au centre.

**Fonctionnement :**
- Calcule la largeur et la hauteur à partir de la zone du terminal.
- Remplit la grille avec des cellules `Empty`.
- Utilise du **bruit de Perlin** pour placer des tuiles `Obstacle` selon un seuil.
- Place un bloc `Base` 3x3 au centre de la carte.
- Répartit aléatoirement les ressources `Energy` et `Crystal` sur les tuiles `Empty` éloignées de la base.

---

### `Map::in_bounds(x: isize, y: isize) -> bool`
**But :** Vérifier si une coordonnée se trouve à l’intérieur des limites de la grille.  
**Fonctionnement :** Retourne `true` si `x` et `y` sont positifs et inférieurs à `width` et `height`.

---

### `Map::is_walkable(cell: &Cell) -> bool`
**But :** Déterminer si une cellule peut être traversée.  
**Fonctionnement :** Retourne `false` uniquement pour `Obstacle`. Toutes les autres cellules sont traversables.

---

### `Map::get_cell(x: usize, y: usize) -> Cell`
**But :** Fournir un accès en lecture seule à une cellule.  
**Fonctionnement :** Lit la cellule depuis la grille protégée par le `RwLock` et renvoie une copie.

---

### `Map::set_cell(x: usize, y: usize, cell: Cell)`
**But :** Fournir un accès en écriture à une cellule.  
**Fonctionnement :** Écrit la nouvelle valeur dans la grille protégée par le `RwLock`.

---

### `Map::try_collect_one(x: usize, y: usize) -> Option<Cell>`
**But :** Retirer une unité d’une ressource présente sur une tuile et renvoyer la ressource collectée.

**Fonctionnement :**
- Si la cellule est `Energy(q > 0)` ou `Crystal(q > 0)`, décrémente `q`.
- Si `q` devient `0`, la cellule devient `Empty`; sinon elle reste `Energy/Crystal(q - 1)`.
- Retourne `Some(Cell::Energy(1))` ou `Some(Cell::Crystal(1))` en cas de succès, sinon `None`.

---

## Utilitaires de pathfinding (A*)

### `Map::heuristic(a, b) -> u32`
**But :** Fonction heuristique pour l’algorithme A* (distance de Manhattan).  
**Fonctionnement :** Retourne `|dx| + |dy|` entre `a` et `b`.

---

### `Map::neighbors(pos) -> Vec<(usize, usize)>`
**But :** Énumérer les voisins accessibles dans les 4 directions (haut, bas, gauche, droite).  
**Fonctionnement :** Vérifie `in_bounds` et `is_walkable` pour chaque direction et retourne les positions valides.

---

### `Map::find_path(from, to) -> Option<Vec<(usize, usize)>>`
**But :** Calculer le plus court chemin (coût = 1 par pas) entre deux positions.

**Fonctionnement :**
- Implémente l’algorithme **A\*** classique :
  - Maintient un ensemble ouvert (`BinaryHeap`), un score de coût (`g_score`) et un tableau `came_from`.
  - Lorsqu’on atteint la destination, reconstitue le chemin complet.
- Retourne un `Vec` de coordonnées incluant le point de départ et d’arrivée, ou `None` si aucun chemin n’est possible.

---

### `Map::next_step_towards(from, to) -> (usize, usize)`
**But :** Fournir la prochaine étape immédiate sur un chemin entre `from` et `to`.  
**Fonctionnement :**
- Appelle `find_path`; si un chemin de longueur > 1 existe, retourne `path[1]`.
- Sinon, retourne `from` (aucun mouvement possible).

---
