// src/bin/poker_tournament_lobby_cli.rs

use poker_engine::domain::chips::Chips;
use poker_engine::domain::{PlayerId, TableId};
use poker_engine::domain::tournament::{TournamentConfig, TournamentStatus, TournamentError};
use poker_engine::tournament::TournamentLobby;

fn main() {
    println!("=== TOURNAMENT LOBBY CLI ===\n");

    let mut lobby = TournamentLobby::new();

    // Турнир 1: классический фриз-аут, 9-max.
    let cfg1 = TournamentConfig {
        name: "Daily 10k 9-max".to_string(),
        starting_stack: Chips(10_000),
        table_size: 9,
        freezeout: true,
        reentry_allowed: false,
        max_players: 90,
        max_reentries_per_player: 0,
    };

    // Турнир 2: дипстек с ре-энтри, 6-max.
    let cfg2 = TournamentConfig {
        name: "Deepstack Re-entry 6-max".to_string(),
        starting_stack: Chips(25_000),
        table_size: 6,
        freezeout: false,
        reentry_allowed: true,
        max_players: 60,
        max_reentries_per_player: 2,
    };

    let t1_id = lobby.create_tournament(cfg1);
    let t2_id = lobby.create_tournament(cfg2);

    println!("Созданы турниры {} и {}\n", t1_id, t2_id);

    // Хелпер для регистрации диапазона игроков.
    let mut register_range = |t_id: u64, range: std::ops::RangeInclusive<PlayerId>| {
        for pid in range {
            if let Err(err) = lobby.register_player(t_id, pid) {
                match err {
                    TournamentError::TournamentFull => {
                        println!("  Турнир {} полон, прекращаем регистрацию.", t_id);
                        break;
                    }
                    TournamentError::TooManyReentries { player_id } => {
                        println!("  Игрок {} превысил лимит ре-энтри в турнире {}.", player_id, t_id);
                    }
                    TournamentError::RegistrationClosed => {
                        println!("  Регистрация закрыта в турнире {}.", t_id);
                        break;
                    }
                    TournamentError::TournamentNotFound { tournament_id } => {
                        println!("  Турнир {} не найден при регистрации.", tournament_id);
                        break;
                    }
                }
            }
        }
    };

    // Регистрируем 20 игроков в первый турнир.
    println!("Регистрируем 20 игроков (1..=20) в турнир id={}", t1_id);
    register_range(t1_id, 1..=20);

    // Регистрируем 5 игроков + отдельный игрок с ре-энтри во второй турнир.
    println!("\nРегистрируем 5 игроков (100..=104) в турнир id={} (с ре-энтри)", t2_id);
    register_range(t2_id, 100..=104);

    println!("  Игрок 200 делает три входа (1 + 2 ре-энтри)");
    for _ in 0..3 {
        if let Err(err) = lobby.register_player(t2_id, 200) {
            println!("    Ошибка при входе игрока 200: {}", err);
        }
    }

    // === ПОСТРОЕНИЕ SEATING'A ===

    // Для первого турнира: столы с id, начиная с 1.
    if let Some(t1) = lobby.get_mut(t1_id) {
        t1.rebuild_seating_from_scratch(1);
    }

    // Для второго турнира: чтобы id столов не пересекались, стартуем, например, с 1001.
    if let Some(t2) = lobby.get_mut(t2_id) {
        t2.rebuild_seating_from_scratch(1001);
    }

    println!("\nТекущее состояние лобби (с рассадкой по столам):\n");

    for (_tid, t) in lobby.all() {
        println!(
            "- id={} | name=\"{}\" | status={:?} | active_players={} / max={} | tables={}",
            t.id,
            t.config.name,
            t.status,
            t.active_player_count(),
            t.config.max_players,
            t.table_ids().len()
        );

        for table_id in t.table_ids() {
            println!("    table_id={}", table_id);
            for reg in t.players_on_table(*table_id) {
                let seat = reg.seat_index.unwrap_or(255);
                println!(
                    "        seat={} | player_id={} | stack={} | entries_used={}",
                    seat,
                    reg.player_id,
                    reg.stack.0,
                    reg.entries_used
                );
            }
        }

        println!();
    }

    println!("=== TOURNAMENT LOBBY CLI DONE ===");
}
