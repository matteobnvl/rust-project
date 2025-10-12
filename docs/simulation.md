# simulation.rs — Lancement de la simulation asynchrone et gestion des handles

Ce module crée le **runtime Tokio**, lance les tâches principales (base et robots), puis retourne un handle simplifié permettant d’arrêter proprement la simulation par la suite.

---

## Types

### `SimulationError`
**But :** Type d’erreur utilisé lors de l’initialisation ou de l’exécution de la simulation.

**Variantes :**
- `Io(std::io::Error)` → Erreur d’entrée/sortie.
- `Spawn` → Erreur générique lors du lancement ou de la construction des tâches.

---

### `type Result<T> = std::result::Result<T, SimulationError>`
**But :** Alias local pratique pour les opérations susceptibles d’échouer (`Result` personnalisé avec `SimulationError`).

---

### `SimHandles`
**But :** Structure possédant le runtime Tokio et les `JoinHandle` des tâches lancées.

**Champs :**
- `rt` → `tokio::runtime::Runtime` : le runtime asynchrone multithread.
- `handles` → `Vec<JoinHandle<()>>` : liste des handles des tâches (actuellement non utilisée directement).

**Méthodes :**

#### `shutdown(self)`
**But :** Arrêter proprement la simulation.  
**Fonctionnement :**  
Consomme `self`, ce qui provoque la destruction du runtime et des handles.  
Les tâches asynchrones sont automatiquement terminées lorsque le runtime est libéré.

---

## Fonctions

### `spawn_simulation(map: &mut Map, base_shared: &BaseShared, robots_shared: &RobotsShared) -> Result<SimHandles>`
**But :** Construire un runtime **Tokio multithread** et lancer les tâches de la base et des robots.

**Fonctionnement :**
1. Construit un runtime multi-thread avec `enable_all()`.
2. Prépare un ensemble initial de robots : **2 éclaireurs** et **2 collecteurs**, positionnés à la base.
3. Clone les instances de `Map`, `BaseShared` et `RobotsShared` pour chaque tâche.
4. À l’intérieur d’un `rt.block_on` :
    - Initialise `RobotsShared` avec la liste initiale des robots.
    - Lance les tâches suivantes :
        - `base_loop(base_shared.clone())`
        - `scout_loop(1, map.clone(), base1, robots_shared.clone())`
        - `scout_loop(2, map.clone(), base2, robots_shared.clone())`
        - `collector_loop(3, map.clone(), base3, robots_shared.clone())`
        - `collector_loop(4, map.clone(), base4, robots_shared.clone())`
5. Retourne `SimHandles { rt, handles }` à l’appelant.

---

## Notes

- La **coordination entre les tâches** (communication, réservations, découvertes, etc.) est gérée dans  
  [`base.rs`](./base.rs) et [`robots.rs`](./robots.rs).
- Ce module se concentre uniquement sur **l’orchestration** et la gestion du cycle de vie des tâches.

---
