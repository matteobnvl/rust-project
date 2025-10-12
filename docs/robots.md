# robots.rs — État partagé et comportements des robots (éclaireurs et collecteurs)

Ce module définit la vision partagée des robots et les deux boucles asynchrones principales qui pilotent la simulation : les **éclaireurs (scouts)** et les **collecteurs**.  
Il fournit également une mémoire d’exploration partagée (cases visitées et frontière BFS) ainsi qu’un mécanisme de **réservation (claim)** pour répartir efficacement les éclaireurs sur la carte.

---

## Types

### `RobotKind`
**But :** Indiquer le rôle d’un robot.  
**Variantes :**
- `Scout` → Éclaireur (explore la carte et découvre des ressources).
- `Collector` → Collecteur (récupère les ressources découvertes et les rapporte à la base).

---

### `RobotState`
**But :** Représenter l’état ou le snapshot d’un robot individuel (pour l’interface ou la coordination).

**Champs :**
- `id` → Identifiant unique du robot.
- `kind` → Type du robot (`RobotKind`).
- `pos` → Position actuelle sur la grille.
- `carrying` → Pour les collecteurs uniquement : `None` ou `Some(Cell::Energy(1)) / Some(Cell::Crystal(1))` si le robot transporte une ressource.

---

### `RobotsShared`
**But :** État partagé et thread-safe des robots et de la mémoire d’exploration des éclaireurs.

**Champs :**
- `inner` → `RwLock<Vec<RobotState>>` : contient l’ensemble des états de robots (pour l’UI et la coordination).
- `visited` → `RwLock<HashSet<(usize, usize)>>` : ensemble des tuiles déjà visitées par un éclaireur.
- `frontier` → `RwLock<VecDeque<(usize, usize)>>` : file des tuiles à explorer (frontière BFS).
- `claimed_targets` → `RwLock<HashSet<(usize, usize)>>` : cibles actuellement revendiquées par un éclaireur, pour éviter les doublons de trajectoire.

---

## Méthodes (`RobotsShared`)

### `new() -> Self`
**But :** Créer un nouvel état partagé vide.  
**Fonctionnement :** Initialise toutes les structures de données et verrous (`RwLock`, `HashSet`, `VecDeque`, etc.).

---

### `set_initial(robots: Vec<RobotState>)`
**But :** Initialiser la liste connue des robots au démarrage de la simulation.  
**Fonctionnement :** Acquiert un verrou en écriture sur `inner` et remplace son contenu.

---

### `snapshot() -> Vec<RobotState>`
**But :** Obtenir une copie de tous les états de robots pour le rendu ou la coordination.  
**Fonctionnement :** Acquiert un verrou en lecture sur `inner` et clone la liste.

---

### `update_pos(id, pos)`
**But :** Mettre à jour la position d’un robot (pour l’interface ou la synchronisation).  
**Fonctionnement :** Acquiert un verrou en écriture sur `inner`, trouve le robot par `id`, et met à jour sa position.

---

### `update_carrying(id, carry)`
**But :** Mettre à jour le statut de transport d’un collecteur.  
**Fonctionnement :** Acquiert un verrou en écriture sur `inner` et modifie le champ correspondant.

---

### `mark_visited(pos) -> bool`
**But :** Marquer une case comme visitée (si elle ne l’était pas déjà).  
**Fonctionnement :** Acquiert un verrou en écriture sur `visited` et insère `pos`.  
Retourne `true` si la case vient d’être ajoutée.

---

### `is_visited(pos) -> bool`
**But :** Vérifier si une case a déjà été visitée.  
**Fonctionnement :** Acquiert un verrou en lecture sur `visited` et vérifie l’appartenance.

---

### `push_frontier_many(items)`
**But :** Ajouter plusieurs tuiles à la frontière BFS, en ignorant les doublons et celles déjà visitées.  
**Fonctionnement :**  
Filtre d’abord la liste `items` contre `visited` sous verrou de lecture, puis insère les nouvelles tuiles dans `frontier` sous verrou d’écriture (évite les interblocages de verrous).

---

### `pop_frontier() -> Option<(usize, usize)>`
**But :** Récupérer et retirer la prochaine tuile à explorer.  
**Fonctionnement :** Acquiert un verrou en écriture sur `frontier` et dépile le premier élément.

---

### `try_claim_target(pos) -> bool`
**But :** Tenter de revendiquer une tuile cible afin qu’un autre éclaireur ne la choisisse pas.  
**Fonctionnement :** Acquiert un verrou en écriture sur `claimed_targets`; si la position n’y est pas déjà, l’insère et retourne `true`.

---

### `release_claim(pos)`
**But :** Libérer une tuile précédemment revendiquée.  
**Fonctionnement :** Acquiert un verrou en écriture sur `claimed_targets` et supprime `pos`.

---

### `is_claimed(pos) -> bool`
**But :** Vérifier si une tuile est actuellement revendiquée par un éclaireur.  
**Fonctionnement :** Acquiert un verrou en lecture sur `claimed_targets`.

---

## Boucles asynchrones

### `scout_loop(id, map, base, robots)`
**But :** Comportement autonome d’un éclaireur : exploration partagée (BFS), navigation A*, et report différé des découvertes.

**Fonctionnement :**
1. **Initialisation :** marque la base comme visitée et ajoute ses voisins accessibles à la frontière.
2. **Sélection de cible (en attente) :**
  - Si `returning_to_base` (retour à la base pour rapporter des découvertes), planifie un trajet vers la base.
  - Sinon, extrait jusqu’à **12 candidats** depuis la frontière, filtre ceux déjà visités, évalue leur distance par rapport à soi et aux autres éclaireurs (pour les répartir), puis tente d’en **revendiquer un** via `robots.try_claim_target`.
  - Réinsère les candidats non choisis dans la frontière.
  - Calcule un chemin A* vers la cible choisie et garde la référence dans `current_target` (pour libération future).
3. **Déplacement :** avance d’une case à chaque tick le long du chemin ; marque les nouvelles cases visitées et ajoute leurs voisins dans la frontière.
4. **Découverte :** en marchant sur une tuile `Energy` ou `Crystal`, ajoute la découverte à `pending_discoveries`, libère la cible, efface le chemin, et planifie un retour à la base.
5. **Rapport à la base :** en arrivant à la base avec des découvertes en attente, envoie un `MessageToBase::Discovery` pour chacune, puis reprend l’exploration.
6. **Anti-blocage :** si le robot ne bouge plus depuis plusieurs ticks, efface le chemin, libère la cible, effectue un petit mouvement aléatoire, et réinsère les voisins dans la frontière.

---

### `collector_loop(id, map, base, robots)`
**But :** Comportement autonome d’un collecteur : exploite les ressources connues par la base et garantit l’exclusivité des réservations (deux collecteurs ne visent jamais la même ressource).

**Fonctionnement :**
1. **Transport de ressource :**
  - Si le collecteur transporte une unité (`carrying`), il suit le chemin A* vers la base.
  - À l’arrivée, envoie `MessageToBase::ReachedBase(unload=Some(cell))`.
2. **Aucune ressource en main et aucune cible locale :**
  - Demande une ressource à la base via `base.get_next_resource()` (réserve la position).
  - Si aucune disponible, attend une découverte via le canal de diffusion (`discovery_tx`) et tente de réserver via `base.try_reserve_resource()`.
3. **Déplacement vers la ressource :**
  - Avance d’une case vers la cible.
  - À l’arrivée, appelle `try_collect_one` :
    - En cas de succès, définit `carrying = Some(cell)` et envoie `MessageToBase::Collected(kind, 1)`.
    - Maintient la réservation pour éviter les doublons.
    - Si la ressource est épuisée, appelle `base.remove_known_resource(target)` puis `base.release_resource(target)` et efface la cible.
4. **Anti-blocage :** si le robot reste immobile trop longtemps, abandonne sa cible et libère la réservation.

---
