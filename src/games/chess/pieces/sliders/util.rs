use crate::{Board, BitBoard, AttackLookup};

pub fn get_moves_ray(mut from: BitBoard, slider: impl Fn(BitBoard) -> BitBoard, can_stop: impl Fn(BitBoard) -> bool) -> BitBoard {
    let mut moves = BitBoard::new();
    loop {
        from = slider(from);
        moves |= &from;

        if can_stop(from) {
            break;
        }
    }

    moves
}

pub fn get_ray_attacks(board: &Board, from: BitBoard, dir: u32, ray_attacks: &AttackLookup, reverse_buffer: u128) -> BitBoard {
    let mut attacks = ray_attacks[from.bitscan_reverse() as usize as usize][dir as usize];
    let blocker = attacks & &board.state.blockers;
    if blocker.is_set() {
        let square = if from >= blocker {
            blocker.bitscan_forward()
        } else {
            blocker.bitscan_reverse()
        };
        attacks ^= &ray_attacks[square as usize][dir as usize];
    }
    return attacks;
}