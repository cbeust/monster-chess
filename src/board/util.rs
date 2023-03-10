use crate::{
    generate_edge_list, Action, BitSet, Edges, Game, HistoryMove, PieceSymbol, UndoMoveError,
};

pub type BitBoard = BitSet<1>;
pub type PieceType = usize;

/// I doubt anyone would be practically creating boards of 4,294,967,296 x 4,294,967,296.
/// However, storing these as u32s makes it much easier to interface the bitboards with (particularly, shifting bits with them.)
pub type Rows = u32;
pub type Cols = u32;

pub struct BoardState {
    /// All Pieces is a BitBoard of all pieces, because keeping this bitboard ready makes it much easier to calculate movement for slider pieces.
    pub all_pieces: BitBoard,
    pub first_move: BitBoard,
    pub pieces: Vec<BitBoard>,
    pub teams: Vec<BitBoard>,

    pub moving_team: u32,
    pub current_turn: u32,

    /// Full Moves is one full move, where each team completes one sub move (or all of their turns)
    pub full_moves: u32,

    /// Sub Moves is one sub move, where a single team completes all of their turns
    pub sub_moves: u32,

    /// A turn is a single movement of a piece. Chess only has one turn, but games like duck chess have two (move the piece, then move the duck)
    pub turns: u32,

    /// Edges is a list of "boundary bitboards" for validating the movement of delta pieces (pieces that move in a fixed way everytime)
    pub edges: Vec<Edges>,
    pub rows: Rows,
    pub cols: Cols,

    pub history: Vec<HistoryMove>,
}

impl BoardState {
    pub fn get_piece_team_board(&self, piece: usize, team: usize) -> BitBoard {
        self.pieces[piece] & &self.teams[team]
    }
}

pub type AttackDirections = Vec<BitBoard>;

/// AttackLookup is indexed by the index of the Most Significant 1-Bit.
///
/// It stores an `AttackDirections` (alias for `Vec<BitBoard>`).
///     For pieces that always move the same way (like Delta Pieces), only the first slot of this AttackDirections is used, because there's no directions.
///     For slider pieces, there are different indexes for specific ray directions of it.

pub type AttackLookup = Vec<AttackDirections>;

pub struct Board {
    pub state: BoardState,
    pub game: Game,
    pub attack_lookup: Vec<AttackLookup>,
}

impl Board {
    pub fn empty(game: Game, teams: u128, (rows, cols): (Rows, Cols)) -> Board {
        let pieces_state = game
            .pieces
            .iter()
            .map(|_| BitBoard::new())
            .collect::<Vec<_>>()
            .clone();

        let mut board = Board {
            attack_lookup: vec![],
            game,
            state: BoardState {
                all_pieces: BitBoard::new(),
                first_move: BitBoard::new(),
                pieces: pieces_state.clone(),
                teams: (0..teams).map(|_| BitBoard::new()).collect::<Vec<_>>(),
                edges: generate_edge_list(rows, cols),
                cols,
                rows,
                history: vec![],
                moving_team: 0,
                current_turn: 0,
                full_moves: 0,
                sub_moves: 0,
                turns: 0,
            },
        };

        board.generate_lookups();

        board
    }

    pub fn get_move_mask(&self, team: u32) -> BitBoard {
        let board_len = self.state.rows * self.state.cols;
        let mut bitboard = BitBoard::new();

        for (ind, board) in self.state.pieces.iter().enumerate() {
            let board = *board & &self.state.teams[team as usize];
            let piece = &self.game.pieces[ind];

            for bit in board.iter_one_bits(board_len as u32) {
                bitboard |= &piece.get_moves(self, BitBoard::from_lsb(bit), team);
            }
        }

        bitboard
    }

    pub fn is_attacking(&self, team: u32, target: BitBoard) -> bool {
        (self.get_move_mask(team) & &target).is_set()
    }

    pub fn generate_moves(&self) -> Vec<Action> {
        let board_len = self.state.rows * self.state.cols;
        let mut actions: Vec<Action> = Vec::with_capacity(board_len as usize);

        let team = self.state.moving_team;

        for (ind, board) in self.state.pieces.iter().enumerate() {
            let board = *board & &self.state.teams[team as usize];
            let piece = &self.game.pieces[ind];

            for bit in board.iter_one_bits(board_len as u32) {
                piece.add_actions(&mut actions, self, bit, team);
            }
        }

        actions
    }

    /*
        Don't use when writing an engine directly; use `generate_moves` and `move_restrictions.is_legal` to avoid extra legality checks during pruning.
    */
    pub fn generate_legal_moves(&mut self) -> Vec<Action> {
        let moves = self.generate_moves();
        let game_restrictions = self.game.move_restrictions.duplicate();
        moves
            .iter()
            .map(|el| el.clone())
            .filter(|el| game_restrictions.is_legal(self, el))
            .collect::<Vec<_>>()
    }

    pub fn get_next_team(&self, mut team: u32) -> u32 {
        team += 1;

        if team >= self.state.teams.len() as u32 {
            0
        } else {
            team
        }
    }

    pub fn get_previous_team(&self, mut team: u32) -> u32 {
        team -= 1;

        if team == u32::MAX {
            (self.state.teams.len() - 1) as u32
        } else {
            team
        }
    }

    pub fn make_move(&mut self, action: &Action) {
        let piece_trait = self.game.pieces[action.piece_type].duplicate();
        piece_trait.make_move(self, action);
    }

    pub fn undo_move(&mut self) -> Result<(), UndoMoveError> {
        let history_move = self.state.history.pop();
        match history_move {
            Some(history_move) => {
                let piece_trait = self.game.pieces[history_move.action.piece_type].duplicate();
                piece_trait.undo_move(self, &history_move);
            }
            None => return Err(UndoMoveError::NoHistoryMoves),
        }

        Ok(())
    }
}
