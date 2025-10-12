# base.rs — Logique de la base et état partagé

Ce module contient l’état partagé de la base ainsi que sa boucle d’événements asynchrone.  
Les collecteurs et les éclaireurs communiquent avec la base via des canaux (channels).  
La base agrège les connaissances (ressources connues), gère les réservations, tient à jour les totaux des ressources collectées et diffuse les découvertes aux collecteurs.

---

## Types

### `MessageToBase`
**But :** Messages envoyés par les robots à la base.

**Variantes :**
- `Discovery { pos, cell }` → Un éclaireur signale la découverte d’une ressource à la position `pos`.
- `Collected { kind, amount }` → Un collecteur informe qu’il a collecté `amount` unités de la ressource `kind` (Énergie / Cristal).
- `ReachedBase { robot_id, unload }` → Un collecteur indique qu’il est arrivé à la base, éventuellement avec une ressource à décharger (`unload`), qui sera ajoutée aux totaux.

---

### `BaseStats`
**But :** Compteurs agrégés maintenus par la base.

**Champs :**
- `energy_total` → Nombre total d’unités d’énergie livrées à la base.
- `crystal_total` → Nombre total d’unités de cristal livrées à la base.

---

### `BaseShared`
**But :** État partagé et thread-safe de la base, accessible depuis plusieurs tâches asynchrones.

**Champs :**
- `stats` → `Mutex<BaseStats>` : Stocke les totaux mis à jour lors des événements `Collected` et `ReachedBase`.
- `known_resources` → `RwLock<VecDeque<((usize, usize), Cell)>>` : File contenant les tuiles de ressources supposées encore présentes.
- `assigned_resources` → `RwLock<Vec<(usize, usize)>>` : Ensemble (sous forme de `Vec`) des positions de ressources actuellement réservées par des collecteurs (évite les doublons).
- `discovery_tx` → `broadcast::Sender<((usize, usize), Cell)>` : Utilisé pour diffuser les découvertes à tous les collecteurs.
- `to_base_tx` → `mpsc::Sender<MessageToBase>` : Canal d’envoi utilisé par les robots pour communiquer avec la base.
- `to_base_rx` → `Mutex<mpsc::Receiver<MessageToBase>>` : Canal de réception, lu par la boucle principale de la base.

---

## Méthodes

### `BaseShared::new() -> Self`
**But :** Construire un nouvel état partagé de la base et ses canaux de communication.  
**Fonctionnement :**  
Crée un canal de diffusion (`broadcast`) pour les découvertes et un canal `mpsc` pour les messages vers la base.  
Initialise les compteurs et listes vides.

---

### `BaseShared::remove_known_resource(pos)`
**But :** Supprimer une position de ressource de la base lorsqu’elle est épuisée ou invalide.  
**Fonctionnement :**  
Ne conserve que les entrées dont la position est différente de `pos` dans `known_resources` et `assigned_resources`.

---

### `BaseShared::get_next_resource() -> Option<((usize, usize), Cell)>`
**But :** Fournir la première ressource non réservée de la liste connue et la réserver de manière atomique.  
**Fonctionnement :**  
Parcourt `known_resources` ; la première position non encore dans `assigned_resources` y est ajoutée, puis renvoyée.  
La ressource reste dans `known_resources` (non retirée), pour pouvoir être réutilisée plus tard si elle est relâchée.

---

### `BaseShared::try_reserve_resource(pos) -> bool`
**But :** Réserver une ressource spécifique si elle n’est pas déjà attribuée.  
**Fonctionnement :**  
Vérifie que `pos` est présente dans `known_resources`, puis qu’elle n’est pas déjà dans `assigned_resources` avant de l’y ajouter et de retourner `true`.

---

### `BaseShared::broadcast_discovery(pos, cell)`
**But :** Diffuser une découverte à tous les collecteurs.  
**Fonctionnement :**  
Appelle `discovery_tx.send((pos, cell))`.  
En cas d’absence d’auditeurs, l’échec est ignoré sans conséquence.

---

### `BaseShared::release_resource(pos)`
**But :** Libérer la réservation d’une ressource pour qu’un autre collecteur puisse la prendre.  
**Fonctionnement :**  
Supprime `pos` de `assigned_resources`.

---

## Tâche asynchrone

### `base_loop(shared: BaseShared)`
**But :** Tâche principale de la base, chargée d’agréger les connaissances et de mettre à jour les totaux.

**Fonctionnement :**
1. Attend les messages provenant de `to_base_rx`.
2. Sur `Discovery` : si la cellule contient de l’énergie ou du cristal, elle est ajoutée à `known_resources` et rediffusée aux collecteurs.
3. Sur `Collected` : incrémente les totaux (`energy_total` ou `crystal_total`) de `amount`. Ne libère pas automatiquement la ressource.
4. Sur `ReachedBase` : si `unload` contient `Some(Cell::Energy/Crystal(1))`, incrémente le total correspondant de `1` (confirmation de livraison).

---
