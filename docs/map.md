# Documentation: src/map.rs

But du module
- Définir le type `Tile` représentant le contenu de chaque case et fournir des utilitaires de génération de carte et de ressources.

Enum Tile
- Wall: Mur, non franchissable.
- Floor: Sol libre.
- Source(u32): Source d’énergie non encore vue par les robots.
- Cristal(u32): Cristal non encore vu par les robots.
- CristalFound(u32): Cristal identifié par un éclaireur (quantité connue, collectable par un collecteur).
- SourceFound(u32): Source identifiée par un éclaireur.
- Base: Case appartenant à la base (zone 3×3 au centre).
- Eclaireur: (utilisé pour rendu/trace éventuel, non posé par défaut dans la carte par update()).
- Collecteur: idem.
- Explored: Case visitée/découverte.

Fonctions
- fn generate_map(width, height) -> Result<Vec<Vec<Tile>>, SimulationError>
  Génère une carte bruitée Perlin. Seuil: bruit < 0.3 => Floor, sinon Wall.
- fn generate_sources_rand(width, height) -> Result<Vec<(u16, u16, Tile)>, SimulationError>
  Parcourt la grille et tire au hasard l’apparition de ressources:
  - 0.5% de chances: `Tile::Source(qty)` avec qty entre 5 et 10.
  - 0.3% de chances: `Tile::Cristal(qty)` avec qty entre 5 et 10.
  Retourne la liste des triples (x, y, Tile) à poser si la case est `Floor`. 