use crate::domain::card::Rank;

/// Битовая маска рангов.
///
/// Используем 13 бит (от 2 до A):
/// бит 0 = двойка, бит 12 = туз.
pub type RankMask = u16;

/// Константы масок для всех возможных стритов (5 подряд).
/// Индексация по "старшей карте" стрита.
///
/// Индексы:
///   0: A-5 (wheel)     : A2345
///   1: 6-2             : 23456
///   2: 7-3             : 34567
///   3: 8-4             : 45678
///   4: 9-5             : 56789
///   5: T-6             : 6789T
///   6: J-7             : 789TJ
///   7: Q-8             : 89TJQ
///   8: K-9             : 9TJQK
///   9: A-T (broadway)  : TJQKA
pub const STRAIGHT_MASKS: [RankMask; 10] = [
    // A2345 (wheel): A,2,3,4,5
    mask_from_ranks(&[Rank::Ace, Rank::Two, Rank::Three, Rank::Four, Rank::Five]),
    // 23456
    mask_from_ranks(&[Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six]),
    // 34567
    mask_from_ranks(&[Rank::Three, Rank::Four, Rank::Five, Rank::Six, Rank::Seven]),
    // 45678
    mask_from_ranks(&[Rank::Four, Rank::Five, Rank::Six, Rank::Seven, Rank::Eight]),
    // 56789
    mask_from_ranks(&[Rank::Five, Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine]),
    // 6789T
    mask_from_ranks(&[Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten]),
    // 789TJ
    mask_from_ranks(&[Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack]),
    // 89TJQ
    mask_from_ranks(&[Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen]),
    // 9TJQK
    mask_from_ranks(&[Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King]),
    // TJQKA (broadway)
    mask_from_ranks(&[Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]),
];

/// Получить битовую маску для одного ранга.
pub fn rank_to_bit(rank: Rank) -> RankMask {
    let idx = (rank as u8).saturating_sub(2); // Rank::Two = 2
    1u16 << idx
}

/// Построить маску из списка рангов.
pub const fn mask_from_ranks(ranks: &[Rank]) -> RankMask {
    let mut mask: RankMask = 0;
    let mut i = 0;
    while i < ranks.len() {
        let r = ranks[i] as u8;
        let idx = r.saturating_sub(2);
        mask |= 1 << idx;
        i += 1;
    }
    mask
}

/// Найти стрит в битовой маске рангов.
/// Возвращает старшую карту стрита, если он есть.
///
/// Особый случай: wheel (A2345) → возвращаем Rank::Five.
pub fn detect_straight(rank_mask: RankMask) -> Option<Rank> {
    // Проверяем от самого сильного (broadway) к слабейшему.
    for (i, sm) in STRAIGHT_MASKS.iter().enumerate().rev() {
        if rank_mask & sm == *sm {
            return Some(match i {
                0 => Rank::Five,  // wheel A2345
                1 => Rank::Six,
                2 => Rank::Seven,
                3 => Rank::Eight,
                4 => Rank::Nine,
                5 => Rank::Ten,
                6 => Rank::Jack,
                7 => Rank::Queen,
                8 => Rank::King,
                9 => Rank::Ace,
                _ => Rank::Five,
            });
        }
    }
    None
}
