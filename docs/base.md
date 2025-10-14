# Documentation: src/base.rs

But du module
- Modélise une « base » centrale recevant des messages des robots et diffusant des informations/statistiques via un canal broadcast.

Messages et types
- enum BaseMessage
  - Discovery { pos: RobotPosition, _tile: Tile }: notification qu’un robot a découvert un `Tile` à une position donnée.
  - Collected { resource: Tile, amount: u32 }: notification qu’une quantité a été collectée pour un type de ressource.
- enum BroadcastMessage
  - NewResource { pos: RobotPosition, _tile: Tile }: diffusion d’une nouvelle ressource connue.
  - BaseStats { energy: u32, crystals: u32 }: diffusion des totaux de la base.
- type SharedBase = Arc<Base>: pointeur partagé vers la base pour usage inter-tâches.

État interne
- struct BaseStateData
  - known_map: HashMap<RobotPosition, Tile> carte/ressources connues par la base.
  - total_energy: u32 cumul d’énergie collectée.
  - total_crystals: u32 cumul de cristaux collectés.
  - tx_broadcast: broadcast::Sender<BroadcastMessage> canal de diffusion.
- struct Base { state: RwLock<BaseStateData> }
  L’état est protégé par un RwLock async pour des lectures concurrentes et écritures séquentielles.

Méthodes de Base
- fn new(tx_broadcast) -> SharedBase
  Construit une base avec des compteurs à 0 et une carte vide.
- async fn run(self: Arc<Self>, mut rx_events: mpsc::Receiver<BaseMessage>)
  Boucle asynchrone recevant les messages de `rx_events`:
  - Discovery: écrit dans `known_map` et envoie un `BroadcastMessage::NewResource`.
  - Collected: met à jour `total_energy`/`total_crystals` selon le type de `Tile` (Source/Cristal) et broadcast `BaseStats`.
- async fn totals(&self) -> (u32, u32)
  Lit l’état et retourne (energy, crystals).

Intégration
- Le module est initialisé dans `main` avec des canaux mpsc/broadcast et sa tâche `run` est spawnée. Pour l’instant, `GameState` ne consomme pas encore ces messages, mais la structure est prête pour des extensions.

Notes
- `src/utils.rs` est explicitement exclu de la documentation détaillée comme demandé.