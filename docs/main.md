# main.rs — Point d’entrée et boucle d’interface (UI)

Ce module met en place l’interface terminal (TUI), construit les états partagés, lance la simulation asynchrone et exécute la boucle d’interface jusqu’à l’appui d’une touche.

---

## Fonctions

### `main() -> Result<()>`
**But :**  
Initialiser la journalisation, le terminal, la carte et les états partagés, lancer la simulation, puis exécuter la boucle d’interface utilisateur.

**Fonctionnement :**
1. Configure le logger via `utils::configure_logger()` et affiche un message de démarrage.
2. Initialise le terminal avec **ratatui** et calcule la zone disponible.
3. Construit la carte (`Map`) adaptée à la taille du terminal via  
   `Map::from_area(Size { width, height })`.
4. Crée les états partagés `BaseShared` et `RobotsShared`.
5. Lance la simulation avec  
   `spawn_simulation(&mut map, &base_shared, &robots_shared)` et conserve les `SimHandles` retournés.
6. Entre dans la boucle de rendu et d’événements via `run_ui_loop(...)`.  
   Lorsque l’utilisateur appuie sur une touche, la boucle se termine.
7. À la sortie, ferme proprement la simulation via `sim_handles.shutdown()`, restaure le terminal et retourne `Ok(())`.

---

### `run_ui_loop(terminal, map, base_shared, robots_shared) -> Result<()>`
**But :**  
Gérer le rendu périodique et détecter les événements de sortie.

**Fonctionnement :**
1. Définit un taux de rafraîchissement (`tick rate`) de **80 ms** et enregistre le temps du dernier tick.
2. Attend un événement clavier avec un timeout correspondant au temps restant avant le prochain tick.  
   Si une touche est pressée → quitte la boucle et retourne `Ok`.
3. Quand le tick expire, met à jour `last_tick` et écrit un message de débogage.
4. Appelle
   ```rust
   terminal.draw(|f| ui::render(f, map, base_shared, robots_shared))
