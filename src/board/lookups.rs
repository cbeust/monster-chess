use super::{pieces::Piece, AttackLookup, BitBoard, Board, Cols, Rows};

pub fn generate_lookups(
    board: &Board,
    piece: &&'static dyn Piece,
    rows: Rows,
    cols: Cols,
) -> AttackLookup {
    let mut lookups = Vec::with_capacity(board.state.squares as usize);

    for i in 0..board.state.squares {
        let from = BitBoard::from_lsb(i);
        lookups.insert(i as usize, piece.generate_lookup_moves(board, from));
    }

    lookups
}

impl<'a> Board<'a> {
    pub fn generate_lookups(&mut self) {
        for (ind, piece) in self.game.pieces.iter().enumerate() {
            if !piece.can_lookup() {
                self.attack_lookup.insert(ind, vec![]);
                continue;
            }

            self.attack_lookup.insert(
                ind,
                generate_lookups(self, piece, self.state.rows, self.state.cols),
            );
        }
    }
}
