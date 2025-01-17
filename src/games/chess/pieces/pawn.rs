use crate::{
    bitset::Direction,
    board::{
        actions::{
            Action, HistoryMove, HistoryState, HistoryUpdate, IndexedPreviousBoard, PreviousBoard,
        },
        edges::Edges,
        pieces::{Piece, PieceSymbol},
        AttackDirections, BitBoard, Board, Cols, PieceType,
    },
    games::chess::game::ATTACKS_MODE,
};

const NORMAL_PAWN_MOVE: usize = 0;
const EN_PASSANT_MOVE: usize = 1;
fn promotion_move(piece_type: PieceType) -> usize {
    piece_type + 2
}

pub struct PawnPiece;

pub fn up(bitboard: &BitBoard, shift: u32, cols: Cols, team: u32) -> BitBoard {
    match team {
        0 => bitboard.up(shift, cols),
        1 => bitboard.down(shift, cols),
        _ => bitboard.up(shift, cols),
    }
}

pub fn down(bitboard: &BitBoard, shift: u32, cols: Cols, team: u32) -> BitBoard {
    match team {
        0 => bitboard.down(shift, cols),
        1 => bitboard.up(shift, cols),
        _ => bitboard.down(shift, cols),
    }
}

impl PawnPiece {
    fn make_en_passant_move(
        &self,
        board: &mut Board,
        action: &Action,
        piece_type: usize,
        from: BitBoard,
        to: BitBoard,
    ) {
        let cols = board.state.cols;

        let color: usize = action.team as usize;
        let en_passant_target = down(&to, 1, cols, color as u32);

        let en_passant_target_color: usize = if (en_passant_target & board.state.teams[0]).is_set()
        {
            0
        } else {
            1
        };

        board.history.push(HistoryMove {
            action: *action,
            state: HistoryState::Any {
                all_pieces: PreviousBoard(board.state.all_pieces),
                first_move: PreviousBoard(board.state.first_move),
                updates: vec![
                    HistoryUpdate::Team(IndexedPreviousBoard(color, board.state.teams[color])),
                    HistoryUpdate::Team(IndexedPreviousBoard(
                        en_passant_target_color,
                        board.state.teams[en_passant_target_color],
                    )),
                    HistoryUpdate::Piece(IndexedPreviousBoard(
                        piece_type,
                        board.state.pieces[piece_type],
                    )),
                ],
            },
        });

        board.state.teams[color] ^= from;
        board.state.teams[color] |= to;
        board.state.teams[en_passant_target_color] ^= en_passant_target;

        board.state.pieces[piece_type] ^= from;
        board.state.pieces[piece_type] ^= en_passant_target;
        board.state.pieces[piece_type] |= to;

        board.state.all_pieces ^= from;
        board.state.all_pieces ^= en_passant_target;
        board.state.all_pieces |= to;

        board.state.first_move &= !from;
        board.state.first_move &= !en_passant_target;
    }
}

impl Piece for PawnPiece {
    fn can_lookup(&self) -> bool {
        true
    }

    fn generate_lookup_moves(&self, board: &Board, mut from: BitBoard) -> AttackDirections {
        let mut attack_dirs: AttackDirections = vec![];
        let edges = board.state.edges[0];
        for team in 0..board.game.teams {
            let from = match team {
                0 => from & !edges.top,
                1 => from & !edges.bottom,
                _ => from & !edges.top,
            };
            let up_one = up(&from, 1, board.state.cols, team);
            let mut captures = (up_one & !edges.right).right(1);
            captures |= (up_one & !edges.left).left(1);
            attack_dirs.push(captures);
        }
        attack_dirs
    }

    fn get_piece_symbol(&self) -> PieceSymbol {
        PieceSymbol::Char('p')
    }

    fn parse_info(&self, board: &Board, info: String) -> u32 {
        if info.is_empty() {
            // TODO: Check for En Passant
            0
        } else {
            if info.len() > 1 {
                panic!("Promotion Piece Types can only be a single char. '{info}' is invalid.")
            }
            let char = info.chars().nth(0).unwrap();
            let piece_type = board
                .game
                .pieces
                .iter()
                .position(|piece_trait| match piece_trait.get_piece_symbol() {
                    PieceSymbol::Char(piece_symbol) => char == piece_symbol,
                    PieceSymbol::TeamSymbol(chars) => chars.contains(&char),
                })
                .expect(&format!(
                    "Could not find a promotion piece type from '{info}'"
                ));
            (piece_type as u32) + 2
        }
    }

    fn format_info(&self, board: &Board, info: usize) -> String {
        if info > 1 {
            let piece_trait = &board.game.pieces[info - 2];
            if let PieceSymbol::Char(char) = piece_trait.get_piece_symbol() {
                char.to_string()
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        }
    }

    fn can_move_mask(
        &self,
        board: &Board,
        from: BitBoard,
        from_bit: u32,
        piece_type: usize,
        team: u32,
        mode: u32,
        to: BitBoard,
    ) -> BitBoard {
        self.get_attack_lookup(board, piece_type).unwrap()[from_bit as usize][team as usize]
    }

    fn get_moves(
        &self,
        board: &Board,
        from: BitBoard,
        piece_type: usize,
        team: u32,
        mode: u32,
    ) -> BitBoard {
        let cols = board.state.cols;
        let edges = &board.state.edges[0];

        if mode == ATTACKS_MODE {
            return self.get_attack_lookup(board, piece_type).unwrap()
                [from.bitscan_forward() as usize][team as usize];
        }

        let mut moves = BitBoard::new();

        let mut capture_requirements = board.state.all_pieces;
        let mut captures = self.get_attack_lookup(board, piece_type).unwrap()
            [from.bitscan_forward() as usize][team as usize];

        let single_moves = up(&from, 1, cols, team) & !board.state.all_pieces;
        let first_move = (from & board.state.first_move).is_set();

        moves |= single_moves;

        if first_move {
            let double_moves = up(&single_moves, 1, cols, team) & !board.state.all_pieces;
            moves |= double_moves;
        }

        if let Some(last_move) = board.history.last() {
            let conditions = last_move.action.piece_type == 0
                && (last_move.action.to.abs_diff(last_move.action.from) == (2 * (cols)));

            if conditions {
                capture_requirements |= up(
                    &BitBoard::from_lsb(last_move.action.from),
                    1,
                    cols,
                    board.get_next_team(team),
                );
            }
        }

        captures &= capture_requirements;

        moves |= captures;

        moves
    }

    fn make_capture_move(
        &self,
        board: &mut Board,
        action: &Action,
        piece_type: usize,
        from: BitBoard,
        to: BitBoard,
    ) {
        let color: usize = action.team as usize;
        let captured_color: usize = if (to & board.state.teams[0]).is_set() {
            0
        } else {
            1
        };
        let mut captured_piece_type: usize = 0;
        for i in 0..(board.game.pieces.len()) {
            if (board.state.pieces[i] & to).is_set() {
                captured_piece_type = i;
                break;
            }
        }

        let mut history_move = HistoryMove {
            action: *action,
            state: HistoryState::Any {
                all_pieces: PreviousBoard(board.state.all_pieces),
                first_move: PreviousBoard(board.state.first_move),
                updates: vec![
                    HistoryUpdate::Team(IndexedPreviousBoard(color, board.state.teams[color])),
                    HistoryUpdate::Team(IndexedPreviousBoard(
                        captured_color,
                        board.state.teams[captured_color],
                    )),
                    HistoryUpdate::Piece(IndexedPreviousBoard(
                        piece_type,
                        board.state.pieces[piece_type],
                    )),
                    HistoryUpdate::Piece(IndexedPreviousBoard(
                        captured_piece_type,
                        board.state.pieces[captured_piece_type],
                    )),
                ],
            },
        };

        let mut promotion_piece_type: Option<usize> = None;
        if action.info >= 2 {
            let promotion_type = action.info - 2;
            promotion_piece_type = Some(promotion_type);
            if let HistoryState::Any { updates, .. } = &mut history_move.state {
                updates.push(HistoryUpdate::Piece(IndexedPreviousBoard(
                    promotion_type,
                    board.state.pieces[promotion_type],
                )));
            }
        }

        board.state.teams[captured_color] ^= to;
        board.state.teams[color] ^= from;
        board.state.teams[color] |= to;

        board.state.pieces[captured_piece_type] ^= to;
        board.state.pieces[piece_type] ^= from;
        match promotion_piece_type {
            None => {
                board.state.pieces[piece_type] |= to;
            }
            Some(promotion_piece_type) => {
                board.state.pieces[promotion_piece_type] |= to;
            }
        }

        board.state.all_pieces ^= from;

        board.state.first_move &= !from;
        board.state.first_move &= !to;
        // We actually don't need to swap the blockers. A blocker will still exist on `to`, just not on `from`.

        board.history.push(history_move);
    }

    fn make_normal_move(
        &self,
        board: &mut Board,
        action: &Action,
        piece_type: usize,
        from: BitBoard,
        to: BitBoard,
    ) {
        if action.info == EN_PASSANT_MOVE {
            self.make_en_passant_move(board, action, piece_type, from, to);
            return;
        }

        let color: usize = action.team as usize;

        board.history.push(HistoryMove {
            action: *action,
            state: HistoryState::Single {
                team: IndexedPreviousBoard(color, board.state.teams[color]),
                piece: IndexedPreviousBoard(piece_type, board.state.pieces[piece_type]),
                all_pieces: PreviousBoard(board.state.all_pieces),
                first_move: PreviousBoard(board.state.first_move),
            },
        });

        if action.info >= 2 {
            let promotion_type = action.info - 2;
            let history_state = &mut board.history.last_mut().unwrap().state;
            *history_state = HistoryState::Any {
                first_move: PreviousBoard(board.state.first_move),
                all_pieces: PreviousBoard(board.state.all_pieces),
                updates: vec![
                    HistoryUpdate::Team(IndexedPreviousBoard(color, board.state.teams[color])),
                    HistoryUpdate::Piece(IndexedPreviousBoard(
                        piece_type,
                        board.state.pieces[piece_type],
                    )),
                    HistoryUpdate::Piece(IndexedPreviousBoard(
                        promotion_type,
                        board.state.pieces[promotion_type],
                    )),
                ],
            };
            board.state.pieces[promotion_type] |= to;
            board.state.pieces[piece_type] ^= from;
        } else {
            board.state.pieces[piece_type] = (board.state.pieces[piece_type] ^ from) | to;
        }

        board.state.teams[color] = (board.state.teams[color] ^ from) | to;

        board.state.all_pieces ^= from;
        board.state.all_pieces |= to;

        //board.state.first_move &= !from;
    }

    fn add_actions(
        &self,
        actions: &mut Vec<Action>,
        board: &Board,
        piece_type: usize,
        from: u32,
        team: u32,
        mode: u32,
    ) {
        let promotion_rows = board.state.edges[0].bottom | board.state.edges[0].top;

        let from_board = BitBoard::from_lsb(from);
        let bit_actions = self.get_moves(board, from_board, piece_type, team, mode)
            & !board.state.teams[team as usize];

        if bit_actions.is_empty() {
            return;
        }

        let cols = board.state.cols;

        let piece_types = board.game.pieces.len();

        for bit in bit_actions.iter_one_bits(board.state.squares) {
            if (BitBoard::from_lsb(bit) & promotion_rows).is_set() {
                for promotion_piece_type in 0..piece_types {
                    if promotion_piece_type == 0 {
                        continue;
                    }
                    if promotion_piece_type == 5 {
                        continue;
                    }
                    actions.push(Action {
                        from,
                        to: bit,
                        team,
                        info: promotion_move(promotion_piece_type),
                        piece_type,
                    });
                }
            } else {
                let mut en_passant = false;
                if let Some(last_move) = board.history.last() {
                    let conditions = last_move.action.piece_type == 0
                        && (last_move.action.to.abs_diff(last_move.action.from) == (2 * (cols)))
                        && (last_move.action.to.abs_diff(bit) == (cols))
                        && (from.abs_diff(bit) % cols != 0);

                    if conditions {
                        en_passant = true;
                    }
                }

                actions.push(Action {
                    from,
                    to: bit,
                    team,
                    info: if en_passant {
                        EN_PASSANT_MOVE
                    } else {
                        NORMAL_PAWN_MOVE
                    },
                    piece_type,
                });
            }
        }
    }
}
