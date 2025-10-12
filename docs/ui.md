# ui.rs — Rendu terminal de la simulation

Ce module gère l’affichage de la simulation dans le terminal à l’aide de **ratatui**.  
Il affiche la carte, les robots, ainsi qu’une barre de titre / d’état contenant les totaux et informations principales.

---

## Fonctions

### `render(f: &mut ratatui::Frame<'_>, map: &Map, base_shared: &BaseShared, robots_shared: &RobotsShared)`
**But :**  
Dessiner l’ensemble de l’interface utilisateur à chaque frame.

**Fonctionnement :**
1. Lit les statistiques de la base (`BaseStats`) et prend un **snapshot** des états de robots (`RobotsShared`).
2. Construit une **barre de titre** ou une **toolbar** affichant les totaux (énergie / cristal) et le nombre de robots.
3. Alloue le rectangle de dessin principal (`inner_map_area`) et génère les lignes de texte représentant la grille :
    - `Empty` → espace vide
    - `Obstacle` → `O` (Cyan)
    - `Energy` → `E` (Vert)
    - `Crystal` → `C` (Magenta)
    - `Base` → `#`
4. Superpose les **robots** sur la carte selon leur position :
    - **Éclaireur (Scout)** → `x` (Rouge)
    - **Collecteur (Collector)** → `o` (Magenta)
5. Rend l’ensemble dans un **Block** encadré (avec bordure et titre éventuel).

---

### `inner_map_area(area: Rect) -> Rect`
**But :**  
Retourner le rectangle utilisé pour dessiner le contenu de la carte.

**Fonctionnement :**  
Retourne actuellement `area` inchangé — le `Block` encadrant gère la bordure externe,  
et la zone interne (`inner_map_area`) occupe tout l’espace disponible à l’intérieur.

---
